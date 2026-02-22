// main.go – Generate a PDF from an HTML file using pdf_forge via cgo.
//
// Build (Linux/macOS):
//
//	cargo build --release   # from the repo root first
//	cd examples/go
//	CGO_LDFLAGS="-L../../target/release -lpdf_forge -lpthread -ldl -lm" \
//	  go build -o generate_pdf .
//	LD_LIBRARY_PATH=../../target/release ./generate_pdf ../../templates/minimal.html out.pdf
//
// Build (Windows – MinGW + GCC on PATH):
//
//	cargo build --release
//	cd examples\go
//	set CGO_LDFLAGS=-L../../target/release -lpdf_forge -lws2_32 -lbcrypt -lntdll -luserenv
//	go build -o generate_pdf.exe .
//	generate_pdf.exe ..\..\templates\minimal.html out.pdf
//
// Optional flags:
//
//	./generate_pdf --title "Q4 Report" --landscape input.html output.pdf

package main

/*
#cgo CFLAGS: -I../../include
#include "rpdf.h"
#include <stdlib.h>
*/
import "C"

import (
	"errors"
	"fmt"
	"os"
	"unsafe"
)

// GeneratePDF converts HTML bytes into a PDF byte slice using the given config.
// title is embedded in the PDF document metadata; pass "" for the default.
// landscape rotates the effective page to A4 landscape when true.
func GeneratePDF(html []byte, title string, landscape bool) ([]byte, error) {
	if len(html) == 0 {
		return nil, errors.New("html must not be empty")
	}

	// Build the C config struct.
	var cfg C.RpdfPipelineConfig

	// Title string: allocate a C string for the duration of the call.
	if title != "" {
		cTitle := C.CString(title)
		defer C.free(unsafe.Pointer(cTitle))
		cfg.title = cTitle
	} // nil → library uses default ("rpdf output")

	if landscape {
		cfg.orientation = C.Landscape
	} else {
		cfg.orientation = C.Portrait
	}
	// page_width, page_height, page_margin left at 0 → A4 defaults

	htmlPtr := (*C.uint8_t)(unsafe.Pointer(&html[0]))
	htmlLen := C.uint32_t(len(html))

	var outBuf *C.uint8_t
	var outLen C.uint32_t

	rc := C.rpdf_generate_pdf_ex(htmlPtr, htmlLen, &cfg, &outBuf, &outLen)
	if rc != 0 {
		errPtr := C.rpdf_last_error()
		if errPtr != nil {
			return nil, fmt.Errorf("rpdf error (code %d): %s", int(rc), C.GoString(errPtr))
		}
		return nil, fmt.Errorf("rpdf_generate_pdf_ex failed with code %d", int(rc))
	}
	defer C.rpdf_free_buffer(outBuf, outLen)

	// Copy the Rust-owned bytes into a Go slice before freeing.
	return C.GoBytes(unsafe.Pointer(outBuf), C.int(outLen)), nil
}

// Version returns the pdf_forge library version string.
func Version() string {
	return C.GoString(C.rpdf_version())
}

func main() {
	// ── Parse args ───────────────────────────────────────────────────────────
	args := os.Args[1:]
	if len(args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: generate_pdf [--title <title>] [--landscape] <input.html> <output.pdf>")
		os.Exit(1)
	}

	title := ""
	landscape := false
	var inputPath, outputPath string

	positional := 0
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--title", "-t":
			if i+1 >= len(args) {
				fmt.Fprintln(os.Stderr, "Error: --title requires a value")
				os.Exit(1)
			}
			i++
			title = args[i]
		case "--landscape", "-l":
			landscape = true
		default:
			switch positional {
			case 0:
				inputPath = args[i]
			case 1:
				outputPath = args[i]
			default:
				fmt.Fprintf(os.Stderr, "Unexpected argument: %s\n", args[i])
				os.Exit(1)
			}
			positional++
		}
	}

	if inputPath == "" || outputPath == "" {
		fmt.Fprintln(os.Stderr, "Usage: generate_pdf [--title <title>] [--landscape] <input.html> <output.pdf>")
		os.Exit(1)
	}

	fmt.Printf("pdf_forge %s\n", Version())

	// ── Read HTML ────────────────────────────────────────────────────────────
	html, err := os.ReadFile(inputPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading %s: %v\n", inputPath, err)
		os.Exit(1)
	}

	// ── Generate PDF ─────────────────────────────────────────────────────────
	pdf, err := GeneratePDF(html, title, landscape)
	if err != nil {
		fmt.Fprintf(os.Stderr, "PDF generation failed: %v\n", err)
		os.Exit(1)
	}

	// ── Write output ─────────────────────────────────────────────────────────
	if err := os.WriteFile(outputPath, pdf, 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing %s: %v\n", outputPath, err)
		os.Exit(1)
	}

	fmt.Printf("Wrote %s (%d bytes)\n", outputPath, len(pdf))
}
