use libloading::{Library, Symbol};
use std::os::raw::c_double;

fn main() {
    let lib = unsafe { Library::new("lib.dll").expect("Failed to load DLL") };
    let radius = 10.0;

    unsafe {
        // Load the CalculateCircleArea function
        let calculate_area: Symbol<unsafe extern "C" fn(c_double) -> c_double> = lib
            .get(b"CalculateCircleArea")
            .expect("Failed to load function");

        let area = calculate_area(radius);
        println!("Circle with radius {} has area {}", radius, area);
    }

    unsafe {
        // Load the FormatCircleInfo function
        let format_info: Symbol<unsafe extern "C" fn(c_double) -> *mut i8> = lib
            .get(b"FormatCircleInfo")
            .expect("Failed to load function");

        let free_string: Symbol<unsafe extern "C" fn(*mut i8)> = lib
            .get(b"FreeString")
            .expect("Failed to load FreeString function");

        let info = format_info(radius);
        let message = std::ffi::CStr::from_ptr(info).to_string_lossy();
        println!("{}", message);

        // Free the allocated string in Go
        free_string(info);
    }
}
