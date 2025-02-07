package main

/*
#include <stdlib.h>
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

func main() {} // Required for a Go shared library
