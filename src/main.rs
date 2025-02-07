use lazy_static::lazy_static;
use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

/// Type alias for the callback function pointer that the shared library expects.
/// (This matches the Go-exported callback type.)
pub type CallbackType = unsafe extern "C" fn(c_double) -> c_double;
/// Callback type expected by the asynchronous function.
type AsyncCallback = unsafe extern "C" fn(c_double, *mut c_void) -> bool;

// Global storage for the callback closure.
// This global variable is protected by a Mutex and allows the trampoline function
// to retrieve the user-provided closure.

lazy_static! {
    static ref CALLBACK_STORE: Mutex<Option<Callback>> = Mutex::new(None);
}
type Callback = Box<dyn Fn(f64) -> f64 + Send>;

/// Enum representing different shape types, matching the C enum.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum ShapeType {
    Circle = 0,
    Square = 1,
    Triangle = 2,
}

/// A shape struct with C layout that can represent different shapes.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Shape {
    pub shape_type: ShapeType,
    pub dimension1: c_double, // radius for circle, side for square, base for triangle
    pub dimension2: c_double, // unused for circle/square, height for triangle
}

/// Define a Rust struct with C layout representing a circle.
/// Deriving Copy and Clone allows us to pass the struct by value.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Circle {
    pub radius: c_double,
}

/// A safe wrapper around the Go circle library that includes callback support.
///
/// This struct loads the shared library and exposes safe methods for calculating
/// the circle area, formatting circle info, and invoking a callback. The `call_callback_with`
/// method allows a Rust closure (e.g. `|x| x * x`) to be used as the callback, hiding all
/// unsafe FFI and pointer operations.
pub struct CircleLibrary {
    // We store the leaked library reference to ensure that the symbols remain valid.
    // Keep the loaded library alive for the lifetime of the wrapper.
    _lib: &'static Library,
    calculate_circle_area: unsafe extern "C" fn(c_double) -> c_double,
    calculate_struct_area: unsafe extern "C" fn(Circle) -> c_double,
    format_circle_info: unsafe extern "C" fn(c_double) -> *mut c_char,
    free_string: unsafe extern "C" fn(*mut c_char),
    call_callback: unsafe extern "C" fn(c_double, CallbackType) -> c_double,
    // Pointer to the asynchronous function.
    calculate_circle_area_async: unsafe extern "C" fn(c_double, AsyncCallback, *mut c_void),
    calculate_circle_area_async_multiple:
        unsafe extern "C" fn(c_double, AsyncCallback, *mut c_void),
    calculate_shape_area: unsafe extern "C" fn(Shape) -> c_double,
}

impl CircleLibrary {
    /// Loads the shared library from the given path.
    ///
    /// # Arguments
    /// * `path` - The file path to the shared library (e.g., "lib.dll).
    ///
    /// # Errors
    /// Returns an error if the library or any symbol fails to load.
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Load the library.
        let lib = unsafe { Library::new(path) }?;
        // Leak the library to obtain a 'static lifetime reference; this is acceptable when the
        // library is intended to remain loaded for the duration of the program.
        let lib: &'static Library = Box::leak(Box::new(lib));

        unsafe {
            // Load the function symbols.
            let calculate_circle_area: Symbol<unsafe extern "C" fn(c_double) -> c_double> =
                lib.get(b"CalculateCircleArea")?;
            // Retrieve the symbol for CalculateCircleArea.
            let calculate_struct_area: libloading::Symbol<
                unsafe extern "C" fn(Circle) -> c_double,
            > = lib.get(b"CalculateCircleStructArea")?;
            let format_circle_info: Symbol<unsafe extern "C" fn(c_double) -> *mut c_char> =
                lib.get(b"FormatCircleInfo")?;
            let free_string: Symbol<unsafe extern "C" fn(*mut c_char)> = lib.get(b"FreeString")?;
            let call_callback: Symbol<unsafe extern "C" fn(c_double, CallbackType) -> c_double> =
                lib.get(b"CallCallback")?;
            let calculate_circle_area_async: Symbol<
                unsafe extern "C" fn(c_double, AsyncCallback, *mut c_void),
            > = lib.get(b"CalculateCircleAreaAsync")?;

            let calculate_circle_area_async_multiple: Symbol<
                unsafe extern "C" fn(c_double, AsyncCallback, *mut c_void),
            > = lib.get(b"CalculateCircleAreaAsyncMultiple")?;

            let calculate_shape_area: Symbol<unsafe extern "C" fn(Shape) -> c_double> =
                lib.get(b"CalculateShapeArea")?;

            Ok(CircleLibrary {
                _lib: lib,
                // Dereference the symbols to store the function pointers.
                calculate_circle_area: *calculate_circle_area,
                calculate_struct_area: *calculate_struct_area,
                format_circle_info: *format_circle_info,
                free_string: *free_string,
                call_callback: *call_callback,
                calculate_circle_area_async: *calculate_circle_area_async,
                calculate_circle_area_async_multiple: *calculate_circle_area_async_multiple,
                calculate_shape_area: *calculate_shape_area,
            })
        }
    }

    /// Calculates the area of a circle given the radius.
    ///
    /// # Arguments
    /// * `radius` - The circle's radius.
    ///
    /// # Returns
    /// The computed area as an `f64`.
    pub fn calculate_circle_area(&self, radius: f64) -> f64 {
        unsafe { (self.calculate_circle_area)(radius) }
    }

    /// A safe method that accepts a reference to a Circle and returns its area.
    pub fn calculate_circle_struct_area(&self, circle: &Circle) -> f64 {
        // The external function expects the struct by value.
        unsafe { (self.calculate_struct_area)(*circle) }
    }

    /// Returns a formatted string with circle information.
    ///
    /// This method handles pointer conversion and memory management internally.
    ///
    /// # Arguments
    /// * `radius` - The circle's radius.
    ///
    /// # Returns
    /// A safe `String` containing the formatted message.
    pub fn format_circle_info(&self, radius: f64) -> Result<String, Box<dyn std::error::Error>> {
        unsafe {
            let c_ptr = (self.format_circle_info)(radius);
            if c_ptr.is_null() {
                return Err("Received null pointer from format_circle_info".into());
            }
            // Convert the C string into a Rust String.
            let c_str = CStr::from_ptr(c_ptr);
            let result = c_str.to_string_lossy().into_owned();
            // Free the allocated string in the Go library.
            (self.free_string)(c_ptr);
            Ok(result)
        }
    }

    /// Calls a callback function using the Go library.
    ///
    /// The callback is provided as an extern "C" function pointer.
    pub fn call_callback(&self, val: f64, callback: CallbackType) -> f64 {
        unsafe { (self.call_callback)(val, callback) }
    }

    /// Calls the shared library’s callback function.
    ///
    /// Instead of forcing the user to provide an `extern "C" fn`, this method accepts
    /// any Rust closure with signature `Fn(f64) -> f64`. Internally, the closure is stored
    /// in a global mutex and an `extern "C"` trampoline is passed to the FFI call.
    ///
    /// This design hides all unsafe details and pointer manipulations from the user.
    /// (This method uses a global Mutex for state storage.)
    pub fn call_callback_with<F>(&self, val: f64, callback: F) -> f64
    where
        F: Fn(f64) -> f64 + Send + 'static,
    {
        // Store the provided closure in a global mutex.
        {
            let mut store = CALLBACK_STORE.lock().unwrap();
            *store = Some(Box::new(callback));
        }
        // Call the FFI function with our trampoline as the callback.
        let result = unsafe { (self.call_callback)(val, trampoline) };
        // Clear the global storage after the callback returns.
        {
            let mut store = CALLBACK_STORE.lock().unwrap();
            *store = None;
        }
        result
    }

    /// Asynchronously calculates the area of a circle.
    ///
    /// This method wraps the Go asynchronous function and returns a Future that resolves
    /// to the computed area. Internally, it creates a oneshot channel and passes a boxed sender
    /// as user data to the Go function.
    pub async fn calculate_circle_area_async(&self, radius: f64) -> f64 {
        let (sender, receiver) = oneshot::channel::<f64>();
        let boxed_sender = Box::new(sender);
        let user_data = Box::into_raw(boxed_sender) as *mut c_void;
        unsafe {
            (self.calculate_circle_area_async)(radius, async_trampoline, user_data);
        }
        // Await the result; if the channel is dropped, return 0.0.
        receiver.await.unwrap_or(0.0)
    }

    /// Calls the asynchronous function which produces multiple callback invocations.
    /// Returns an mpsc::UnboundedReceiver that yields each result.
    pub fn calculate_circle_area_async_multi(&self, radius: f64) -> mpsc::UnboundedReceiver<f64> {
        // Create an unbounded channel.
        let (tx, rx) = mpsc::unbounded_channel();
        // Create a new sender for each callback
        let tx = Arc::new(Mutex::new(tx));
        // Convert the Arc into a raw pointer.
        let user_data = Box::into_raw(Box::new(tx)) as *mut c_void;
        unsafe {
            (self.calculate_circle_area_async_multiple)(radius, async_trampoline_multi, user_data);
        }
        rx
    }

    /// Calculate the area of any shape using the shape enum
    pub fn calculate_shape_area(&self, shape: &Shape) -> f64 {
        unsafe { (self.calculate_shape_area)(*shape) }
    }
}

/// Extern "C" trampoline function that matches the expected callback signature.
/// It locks the global storage to retrieve the user’s closure and calls it.
extern "C" fn trampoline(val: c_double) -> c_double {
    let callback_opt = CALLBACK_STORE.lock().unwrap();
    if let Some(ref cb) = *callback_opt {
        cb(val)
    } else {
        0.0 // Default return value if no callback is set.
    }
}

/// Extern "C" trampoline for asynchronous callbacks that supports multiple shots.
/// It recovers the Arc-wrapped sender and sends each callback result.
/// Returns true to continue receiving callbacks, false when done.
unsafe extern "C" fn async_trampoline_multi(result: c_double, user_data: *mut c_void) -> bool {
    // Convert the raw pointer back to a reference.
    let tx = &*(user_data as *const Arc<Mutex<mpsc::UnboundedSender<f64>>>);
    // Attempt to send the result (ignore errors if the receiver is dropped).
    if let Ok(tx) = tx.lock() {
        match tx.send(result) {
            Ok(_) => (),
            Err(e) => println!("Rust: Failed to send result: {}", e),
        }
    }
    // Return false on the last callback (we know there will be 3 callbacks)
    static mut CALLBACK_COUNT: u32 = 0;
    CALLBACK_COUNT += 1;
    let continue_receiving = CALLBACK_COUNT < 3;
    continue_receiving
}

/// Extern "C" trampoline for asynchronous callbacks.
/// This function recovers the boxed oneshot sender from the user data and sends the result.
unsafe extern "C" fn async_trampoline(result: c_double, user_data: *mut c_void) -> bool {
    let boxed_sender: Box<oneshot::Sender<f64>> = Box::from_raw(user_data as *mut _);
    let _ = boxed_sender.send(result);
    false // This is a one-shot callback, so we're done after sending
}

/// A safe wrapper around the Go number generator
pub struct NumberGenerator<'lib> {
    id: i64,
    lib: &'lib Library,
}

// Implement Drop to ensure we clean up the Go resources
impl<'lib> Drop for NumberGenerator<'lib> {
    fn drop(&mut self) {
        self.free_generator();
    }
}

impl<'lib> NumberGenerator<'lib> {
    pub fn new(lib: &'lib Library) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let create_generator: Symbol<unsafe extern "C" fn() -> i64> =
                lib.get(b"CreateNumberGenerator")?;
            let id = create_generator();
            Ok(NumberGenerator { id, lib })
        }
    }

    pub fn next(&self) -> Result<Option<i32>, Box<dyn std::error::Error>> {
        unsafe {
            let get_next: Symbol<unsafe extern "C" fn(i64) -> (c_int, bool)> =
                self.lib.get(b"GetNextNumber")?;
            let (num, ok) = get_next(self.id);
            if ok {
                Ok(Some(num as i32))
            } else {
                Ok(None)
            }
        }
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let stop_generator: Symbol<unsafe extern "C" fn(i64)> =
                self.lib.get(b"StopNumberGenerator")?;
            stop_generator(self.id);
            Ok(())
        }
    }

    fn free_generator(&self) {
        unsafe {
            if let Ok(free_generator) = self
                .lib
                .get::<unsafe extern "C" fn(i64)>(b"FreeNumberGenerator")
            {
                free_generator(self.id);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure that "lib.dll" is in the same directory as the binary or adjust the path accordingly.
    let circle_lib = CircleLibrary::new("lib.dll")?;

    let radius = 10.0;
    let area = circle_lib.calculate_circle_area(radius);
    println!("Synchronous area: {}", area);

    let circle = Circle { radius };
    let struct_area = circle_lib.calculate_circle_struct_area(&circle);
    println!("Struct-based area: {}", struct_area);

    let info = circle_lib.format_circle_info(radius)?;
    println!("{}", info);

    let cb_result = circle_lib.call_callback(5.0, square_callback as CallbackType);
    println!("Callback result (square of 5.0): {}", cb_result);

    let cb_result_closure = circle_lib.call_callback_with(5.0, |x| x * x);
    println!(
        "Callback result with closure (square of 5.0): {}",
        cb_result_closure
    );

    // Demonstrate the asynchronous function.
    println!("Calling asynchronous one-shot calculation...");
    let async_area = circle_lib.calculate_circle_area_async(radius).await;
    println!("Asynchronous area for radius {}: {}", radius, async_area);

    println!("Calling asynchronous multi-shot calculation...");
    let mut rx = circle_lib.calculate_circle_area_async_multi(radius);

    // Create a shorter timeout for testing
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(4));
    tokio::pin!(timeout);

    println!("Rust: Starting to receive results...");
    // Receive multiple callback results as they arrive, with a timeout
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Some(async_area) => {
                        println!("Asynchronous multi-shot area: {}", async_area);
                    }
                    None => {
                        println!("Rust: Channel closed, all results received");
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                println!("Rust: Timeout waiting for results");
                break;
            }
        }
    }
    println!("Rust: Finished receiving results");

    // Example using the Shape enum and struct
    let circle_shape = Shape {
        shape_type: ShapeType::Circle,
        dimension1: 5.0, // radius
        dimension2: 0.0, // unused for circle
    };
    println!(
        "Circle area using Shape enum: {}",
        circle_lib.calculate_shape_area(&circle_shape)
    );

    let triangle_shape = Shape {
        shape_type: ShapeType::Triangle,
        dimension1: 4.0, // base
        dimension2: 3.0, // height
    };
    println!(
        "Triangle area using Shape enum: {}",
        circle_lib.calculate_shape_area(&triangle_shape)
    );

    // Example using Go channels through the number generator
    println!("\nTesting Go channels with number generator:");
    let generator = NumberGenerator::new(&circle_lib._lib)?;

    // Get the first 5 numbers
    for _ in 0..5 {
        if let Ok(Some(num)) = generator.next() {
            println!("Received number: {}", num);
        }
    }

    // Stop the generator
    generator.stop()?;
    println!("Number generator stopped");

    Ok(())
}

/// An example callback function that squares its input.
/// Must have the `extern "C"` calling convention.
extern "C" fn square_callback(val: c_double) -> c_double {
    val * val
}
