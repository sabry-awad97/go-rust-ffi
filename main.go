package main

/*
#include <stdlib.h>

// Define a callback type that takes a double and returns a double.
typedef double (*callback_t)(double);

// Define an async callback type that takes a double result and a user data pointer.
// Returns true if more callbacks are expected, false if this is the last callback.
typedef _Bool (*async_callback_t)(double result, void* userData);

// A helper function that calls the provided synchronous callback.
static double call_callback(callback_t cb, double val) {
    return cb(val);
}

// A helper function that calls the provided async callback.
// Returns true if more callbacks are expected, false if this is the last callback.
static _Bool call_async_callback(async_callback_t cb, double result, void* userData) {
    return cb(result, userData);
}

// Define a Circle struct with a radius field.
typedef struct {
    double radius;
} Circle;

// Define Shape enum type and values
typedef enum {
    SHAPE_CIRCLE = 0,
    SHAPE_SQUARE = 1,
    SHAPE_TRIANGLE = 2
} ShapeType;

// Define a Shape struct that includes the type and dimensions
typedef struct {
    ShapeType shape_type;
    double dimension1; // radius for circle, side for square, base for triangle
    double dimension2; // unused for circle/square, height for triangle
} Shape;
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

//export CalculateCircleAreaAsyncMultiple
func CalculateCircleAreaAsyncMultiple(radius C.double, cb C.async_callback_t, userData unsafe.Pointer) {
    // Spawn a goroutine that calls the callback multiple times.
    go func(r C.double, cb C.async_callback_t, userData unsafe.Pointer) {
        // For example, call the callback three times (simulate multiple events).
        for i := 0; i < 3; i++ {
            time.Sleep(1 * time.Second)
            // Calculate the area (same value each time in this example).
            area := C.double(math.Pi * float64(r) * float64(r))
            // Use the helper function to call the callback.
            // If this is the last callback (i == 2), return false to signal completion
            shouldContinue := bool(C.call_async_callback(cb, area, userData))
            if !shouldContinue {
                break
            }
        }
    }(radius, cb, userData)
}

//export CalculateShapeArea
func CalculateShapeArea(shape C.Shape) C.double {
    switch shape.shape_type {
    case C.SHAPE_CIRCLE:
        return C.double(math.Pi * float64(shape.dimension1) * float64(shape.dimension1))
    case C.SHAPE_SQUARE:
        return C.double(float64(shape.dimension1) * float64(shape.dimension1))
    case C.SHAPE_TRIANGLE:
        return C.double(0.5 * float64(shape.dimension1) * float64(shape.dimension2))
    default:
        return 0.0
    }
}

func main() {} // Required for a Go shared library
