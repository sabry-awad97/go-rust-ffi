use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_double};

/// Type alias for the callback function pointer matching Go's `callback_t`
pub type CallbackType = unsafe extern "C" fn(c_double) -> c_double;

/// A safe wrapper around the Go circle library.
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

    Ok(())
}

/// An example callback function that squares its input.
/// Must have the `extern "C"` calling convention.
extern "C" fn square_callback(val: c_double) -> c_double {
    val * val
}
