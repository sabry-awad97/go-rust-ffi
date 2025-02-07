use lazy_static::lazy_static;
use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_double};
use std::sync::Mutex;

/// Type alias for the callback function pointer that the shared library expects.
/// (This matches the Go-exported callback type.)
pub type CallbackType = unsafe extern "C" fn(c_double) -> c_double;

// Global storage for the callback closure.
// This global variable is protected by a Mutex and allows the trampoline function
// to retrieve the user-provided closure. (See :contentReference[oaicite:2]{index=2} and :contentReference[oaicite:3]{index=3})
lazy_static! {
    static ref CALLBACK_STORE: Mutex<Option<Box<dyn Fn(f64) -> f64 + Send>>> = Mutex::new(None);
}

/// A safe wrapper around the Go circle library that includes callback support.
///
/// This struct loads the shared library and exposes safe methods for calculating
/// the circle area, formatting circle info, and invoking a callback. The `call_callback_with`
/// method allows a Rust closure (e.g. `|x| x * x`) to be used as the callback, hiding all
/// unsafe FFI and pointer operations.
pub struct CircleLibrary {
    // We store the leaked library reference to ensure that the symbols remain valid.
    _lib: &'static Library,
    calculate_circle_area: unsafe extern "C" fn(c_double) -> c_double,
    format_circle_info: unsafe extern "C" fn(c_double) -> *mut c_char,
    free_string: unsafe extern "C" fn(*mut c_char),
    call_callback: unsafe extern "C" fn(c_double, CallbackType) -> c_double,
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
        // Leak the library to obtain a 'static reference.
        let lib: &'static Library = Box::leak(Box::new(lib));

        unsafe {
            // Load the function symbols.
            let calculate_circle_area: Symbol<unsafe extern "C" fn(c_double) -> c_double> =
                lib.get(b"CalculateCircleArea")?;
            let format_circle_info: Symbol<unsafe extern "C" fn(c_double) -> *mut c_char> =
                lib.get(b"FormatCircleInfo")?;
            let free_string: Symbol<unsafe extern "C" fn(*mut c_char)> = lib.get(b"FreeString")?;
            let call_callback: Symbol<unsafe extern "C" fn(c_double, CallbackType) -> c_double> =
                lib.get(b"CallCallback")?;

            Ok(CircleLibrary {
                _lib: lib,
                // Dereference the symbols to store the function pointers.
                calculate_circle_area: *calculate_circle_area,
                format_circle_info: *format_circle_info,
                free_string: *free_string,
                call_callback: *call_callback,
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
    /// in a global mutex and an `extern "C"` trampoline (see below) is passed to the FFI call.
    ///
    /// This design hides all unsafe details and pointer manipulations from the user.
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure that "lib.dll" is in the same directory as the binary or adjust the path accordingly.
    let circle_lib = CircleLibrary::new("lib.dll")?;

    let radius = 10.0;
    let area = circle_lib.calculate_circle_area(radius);
    println!("Calculated area: {}", area);

    let info = circle_lib.format_circle_info(radius)?;
    println!("{}", info);

    // Call the callback via the Go library.
    let callback_result = circle_lib.call_callback(5.0, square_callback as CallbackType);
    println!("Callback result (square of 5.0): {}", callback_result);

    // Now call the callback function by supplying a Rust closure.
    // Here, the closure simply computes the square of its input.
    let call_with_result = circle_lib.call_callback_with(5.0, |x| x * x);
    println!("Callback result (square of 5.0): {}", call_with_result);

    Ok(())
}

/// An example callback function that squares its input.
/// Must have the `extern "C"` calling convention.
extern "C" fn square_callback(val: c_double) -> c_double {
    val * val
}
