package main

/*
#include <stdlib.h>

// Define a callback type that takes a double and returns a double.
typedef double (*callback_t)(double);

// A helper function that calls the provided callback.
static double call_callback(callback_t cb, double val) {
    return cb(val);
}
*/
import "C"

import (
	"fmt"
	"math"
	"unsafe"
)

//export CalculateCircleArea
func CalculateCircleArea(radius C.double) C.double {
	return C.double(math.Pi * float64(radius) * float64(radius))
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

func main() {} // Required for a Go shared library
