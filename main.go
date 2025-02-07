package main

/*
#include <stdlib.h>

// Define a callback type that takes a double and returns a double.
typedef double (*callback_t)(double);

// Define an async callback type that takes a double result and a user data pointer.
typedef void (*async_callback_t)(double result, void* userData);

// A helper function that calls the provided synchronous callback.
static double call_callback(callback_t cb, double val) {
    return cb(val);
}

// A helper function that calls the provided async callback.
static void call_async_callback(async_callback_t cb, double result, void* userData) {
    cb(result, userData);
}

// Define a Circle struct with a radius field.
typedef struct {
    double radius;
} Circle;
*/
import "C"
import (
	"fmt"
	"math"
	"time"
	"unsafe"
)

//export CalculateCircleArea
func CalculateCircleArea(radius C.double) C.double {
	return C.double(math.Pi * float64(radius) * float64(radius))
}

//export CalculateCircleStructArea
func CalculateCircleStructArea(c C.Circle) C.double {
	// Convert the C.double field to a Go float64.
	radius := float64(c.radius)
	area := math.Pi * radius * radius
	return C.double(area)
}

//export FormatCircleInfo
func FormatCircleInfo(radius C.double) *C.char {
	area := CalculateCircleArea(radius)
	result := fmt.Sprintf("Circle with radius %.2f has area %.2f", radius, area)
	return C.CString(result)
}

//export FreeString
func FreeString(str *C.char) {
	C.free(unsafe.Pointer(str))
}

//export CallCallback
func CallCallback(val C.double, cb C.callback_t) C.double {
	return C.call_callback(cb, val)
}

//export CalculateCircleAreaAsync
func CalculateCircleAreaAsync(radius C.double, cb C.async_callback_t, userData unsafe.Pointer) {
	go func(r C.double, cb C.async_callback_t, userData unsafe.Pointer) {
		// Simulate asynchronous delay.
		time.Sleep(1 * time.Second)
		area := C.double(math.Pi * float64(r) * float64(r))
		// Instead of converting the function pointer, call the helper C function.
		C.call_async_callback(cb, area, userData)
	}(radius, cb, userData)
}

func main() {} // Required for a Go shared library
