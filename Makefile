# Makefile for building the Go shared library

# Output library name
OUTPUT_LIB = lib.dll

# Go source file
GO_SOURCE = main.go

.PHONY: all build clean run

# Default target
all: build

# Build the shared library
build:
	go build -o $(OUTPUT_LIB) -buildmode=c-shared $(GO_SOURCE)

# Run the Rust project using Cargo
run:
	cargo run

# Clean up build artifacts
clean:
	del /F /Q $(OUTPUT_LIB) $(OUTPUT_LIB:.dll=.h)
