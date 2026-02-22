//! C-compatible FFI API for cross-language bindings.
//!
//! # ABI Contract
//!
//! All exported functions use `extern "C"` calling convention and `#[no_mangle]`
//! to ensure stable symbol names.
//!
//! ## Memory management
//! - Buffers returned by `rpdf_*` functions are allocated on the Rust heap.
//! - Callers **must** free them with `rpdf_free_buffer` / `rpdf_free_string`.
//! - Passing a null pointer to a free function is a no-op.
//!
//! ## Error handling
//! - Functions that can fail return a `c_int` (0 = success, non-zero = error).
//! - Error details can be retrieved via `rpdf_last_error`.
//!
//! ## Thread safety
//! - The `rpdf_last_error` uses a thread-local, so it is safe to call from
//!   multiple threads.
//!
//! ## Usage from Go (cgo)
//! ```go
//! // #cgo LDFLAGS: -lrpdf
//! // #include <stdint.h>
//! // extern int rpdf_generate_pdf(const char* html, uint32_t html_len,
//! //                               uint8_t** out_buf, uint32_t* out_len);
//! // extern void rpdf_free_buffer(uint8_t* buf, uint32_t len);
//! // extern const char* rpdf_last_error();
//! // extern void rpdf_free_string(char* s);
//! import "C"
//! ```

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;

use crate::pipeline::{generate_pdf, PipelineConfig, PageOrientation};

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

// ---------------------------------------------------------------------------
// C-compatible configuration types
// ---------------------------------------------------------------------------

/// Page orientation for use in [`RpdfPipelineConfig`].
#[repr(C)]
pub enum RpdfPageOrientation {
    /// Portrait mode: height > width (default).
    Portrait = 0,
    /// Landscape mode: width > height.
    Landscape = 1,
}

/// Optional configuration for PDF generation passed to the `*_ex` functions.
///
/// Fields set to `0` (or `NULL` for `title`) fall back to their A4 defaults:
/// - `page_width`  → 595.28 pt
/// - `page_height` → 841.89 pt
/// - `page_margin` → 40 pt
/// - `title`       → "rpdf output"
#[repr(C)]
pub struct RpdfPipelineConfig {
    /// Null-terminated UTF-8 document title embedded in PDF metadata.
    /// Pass `NULL` to use the default title ("rpdf output").
    pub title: *const c_char,
    /// Page width in points. Pass `0.0` to use the default (A4 = 595.28).
    pub page_width: f32,
    /// Page height in points. Pass `0.0` to use the default (A4 = 841.89).
    pub page_height: f32,
    /// Page margin in points. Pass `0.0` to use the default (40 pt).
    pub page_margin: f32,
    /// Page orientation (portrait = 0, landscape = 1).
    pub orientation: RpdfPageOrientation,
}

/// Convert an `RpdfPipelineConfig` (FFI) to a `PipelineConfig` (Rust).
///
/// # Safety
/// `cfg.title`, if non-null, must point to a valid null-terminated UTF-8 string.
unsafe fn pipeline_config_from_c(cfg: &RpdfPipelineConfig) -> PipelineConfig {
    let defaults = PipelineConfig::default();

    let title = if cfg.title.is_null() {
        defaults.title.clone()
    } else {
        CStr::from_ptr(cfg.title)
            .to_str()
            .unwrap_or("rpdf output")
            .to_string()
    };

    let page_width = if cfg.page_width == 0.0 { defaults.page_width } else { cfg.page_width };
    let page_height = if cfg.page_height == 0.0 { defaults.page_height } else { cfg.page_height };
    let page_margin = if cfg.page_margin == 0.0 { defaults.page_margin } else { cfg.page_margin };

    let orientation = match cfg.orientation {
        RpdfPageOrientation::Portrait => PageOrientation::Portrait,
        RpdfPageOrientation::Landscape => PageOrientation::Landscape,
    };

    PipelineConfig { title, page_width, page_height, page_margin, orientation }
}

// ---------------------------------------------------------------------------
// Core API
// ---------------------------------------------------------------------------

/// Generate a PDF from an HTML template string.
///
/// # Parameters
/// - `html_ptr`: pointer to UTF-8 HTML bytes (not necessarily null-terminated)
/// - `html_len`: length of the HTML data in bytes
/// - `out_buf`: on success, receives a pointer to heap-allocated PDF bytes
/// - `out_len`: on success, receives the length of the PDF buffer
///
/// # Returns
/// `0` on success, non-zero on error. On error, call `rpdf_last_error`.
///
/// # Safety
/// - `html_ptr` must point to `html_len` valid bytes.
/// - `out_buf` and `out_len` must be valid pointers.
/// - The caller must free `*out_buf` by calling `rpdf_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_generate_pdf(
    html_ptr: *const u8,
    html_len: u32,
    out_buf: *mut *mut u8,
    out_len: *mut u32,
) -> c_int {
    if html_ptr.is_null() || out_buf.is_null() || out_len.is_null() {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    match generate_pdf(html, &PipelineConfig::default()) {
        Ok((pdf_bytes, _config)) => {
            let len = pdf_bytes.len() as u32;
            let buf = pdf_bytes.into_boxed_slice();
            let raw = Box::into_raw(buf) as *mut u8;
            *out_buf = raw;
            *out_len = len;
            0
        }
        Err(e) => {
            set_last_error(&e);
            3
        }
    }
}

/// Generate a PDF and also return the layout config JSON.
///
/// # Parameters
/// - `html_ptr`, `html_len`: the HTML input
/// - `out_pdf_buf`, `out_pdf_len`: PDF output
/// - `out_json_ptr`: receives a pointer to a null-terminated JSON string
///
/// # Returns
/// `0` on success.
///
/// # Safety
/// Same as `rpdf_generate_pdf`. Additionally, `*out_json_ptr` must be freed
/// with `rpdf_free_string`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_generate_pdf_with_layout(
    html_ptr: *const u8,
    html_len: u32,
    out_pdf_buf: *mut *mut u8,
    out_pdf_len: *mut u32,
    out_json_ptr: *mut *mut c_char,
) -> c_int {
    if html_ptr.is_null() || out_pdf_buf.is_null() || out_pdf_len.is_null() || out_json_ptr.is_null()
    {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    match generate_pdf(html, &PipelineConfig::default()) {
        Ok((pdf_bytes, layout_config)) => {
            // PDF bytes
            let len = pdf_bytes.len() as u32;
            let buf = pdf_bytes.into_boxed_slice();
            let raw = Box::into_raw(buf) as *mut u8;
            *out_pdf_buf = raw;
            *out_pdf_len = len;

            // JSON string
            let json = layout_config.to_json();
            match CString::new(json) {
                Ok(cs) => {
                    *out_json_ptr = cs.into_raw();
                }
                Err(_) => {
                    *out_json_ptr = ptr::null_mut();
                }
            }

            0
        }
        Err(e) => {
            set_last_error(&e);
            3
        }
    }
}

/// Compute only the layout config (no PDF rendering). Returns JSON.
///
/// # Parameters
/// - `html_ptr`, `html_len`: the HTML input
/// - `out_json_ptr`: receives a pointer to a null-terminated JSON string
///
/// # Returns
/// `0` on success.
#[no_mangle]
pub unsafe extern "C" fn rpdf_compute_layout(
    html_ptr: *const u8,
    html_len: u32,
    out_json_ptr: *mut *mut c_char,
) -> c_int {
    if html_ptr.is_null() || out_json_ptr.is_null() {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    let config = crate::pipeline::compute_layout_config(html, &PipelineConfig::default());
    let json = config.to_json();

    match CString::new(json) {
        Ok(cs) => {
            *out_json_ptr = cs.into_raw();
            0
        }
        Err(_) => {
            set_last_error("JSON contained null byte");
            3
        }
    }
}

// ---------------------------------------------------------------------------
// Config-aware variants (*_ex)
// ---------------------------------------------------------------------------

/// Generate a PDF from HTML with a custom [`RpdfPipelineConfig`].
///
/// # Parameters
/// - `html_ptr`, `html_len`: UTF-8 HTML input
/// - `cfg`: optional pointer to an [`RpdfPipelineConfig`]; pass `NULL` for defaults
/// - `out_buf`, `out_len`: PDF output
///
/// # Returns
/// `0` on success.
///
/// # Safety
/// - `html_ptr` must point to `html_len` valid bytes.
/// - `cfg`, if non-null, must be a valid pointer to a fully-initialised
///   [`RpdfPipelineConfig`] whose `title` field (if non-null) is a valid
///   null-terminated UTF-8 string.
/// - The caller must free `*out_buf` with `rpdf_free_buffer`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_generate_pdf_ex(
    html_ptr: *const u8,
    html_len: u32,
    cfg: *const RpdfPipelineConfig,
    out_buf: *mut *mut u8,
    out_len: *mut u32,
) -> c_int {
    if html_ptr.is_null() || out_buf.is_null() || out_len.is_null() {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    let config = if cfg.is_null() {
        PipelineConfig::default()
    } else {
        pipeline_config_from_c(&*cfg)
    };

    match generate_pdf(html, &config) {
        Ok((pdf_bytes, _)) => {
            let len = pdf_bytes.len() as u32;
            let buf = pdf_bytes.into_boxed_slice();
            *out_buf = Box::into_raw(buf) as *mut u8;
            *out_len = len;
            0
        }
        Err(e) => {
            set_last_error(&e);
            3
        }
    }
}

/// Generate a PDF and layout JSON from HTML with a custom [`RpdfPipelineConfig`].
///
/// # Parameters
/// - `html_ptr`, `html_len`: UTF-8 HTML input
/// - `cfg`: optional pointer to an [`RpdfPipelineConfig`]; pass `NULL` for defaults
/// - `out_pdf_buf`, `out_pdf_len`: PDF output (free with `rpdf_free_buffer`)
/// - `out_json_ptr`: layout JSON output (free with `rpdf_free_string`)
///
/// # Returns
/// `0` on success.
///
/// # Safety
/// Same as `rpdf_generate_pdf_ex`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_generate_pdf_with_layout_ex(
    html_ptr: *const u8,
    html_len: u32,
    cfg: *const RpdfPipelineConfig,
    out_pdf_buf: *mut *mut u8,
    out_pdf_len: *mut u32,
    out_json_ptr: *mut *mut c_char,
) -> c_int {
    if html_ptr.is_null()
        || out_pdf_buf.is_null()
        || out_pdf_len.is_null()
        || out_json_ptr.is_null()
    {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    let config = if cfg.is_null() {
        PipelineConfig::default()
    } else {
        pipeline_config_from_c(&*cfg)
    };

    match generate_pdf(html, &config) {
        Ok((pdf_bytes, layout_config)) => {
            let len = pdf_bytes.len() as u32;
            let buf = pdf_bytes.into_boxed_slice();
            *out_pdf_buf = Box::into_raw(buf) as *mut u8;
            *out_pdf_len = len;

            let json = layout_config.to_json();
            match CString::new(json) {
                Ok(cs) => *out_json_ptr = cs.into_raw(),
                Err(_) => *out_json_ptr = ptr::null_mut(),
            }
            0
        }
        Err(e) => {
            set_last_error(&e);
            3
        }
    }
}

/// Compute only the layout config JSON from HTML with a custom [`RpdfPipelineConfig`].
///
/// # Parameters
/// - `html_ptr`, `html_len`: UTF-8 HTML input
/// - `cfg`: optional pointer to an [`RpdfPipelineConfig`]; pass `NULL` for defaults
/// - `out_json_ptr`: layout JSON output (free with `rpdf_free_string`)
///
/// # Returns
/// `0` on success.
///
/// # Safety
/// Same as `rpdf_generate_pdf_ex`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_compute_layout_ex(
    html_ptr: *const u8,
    html_len: u32,
    cfg: *const RpdfPipelineConfig,
    out_json_ptr: *mut *mut c_char,
) -> c_int {
    if html_ptr.is_null() || out_json_ptr.is_null() {
        set_last_error("Null pointer argument");
        return 1;
    }

    let html_bytes = slice::from_raw_parts(html_ptr, html_len as usize);
    let html = match std::str::from_utf8(html_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {e}"));
            return 2;
        }
    };

    let config = if cfg.is_null() {
        PipelineConfig::default()
    } else {
        pipeline_config_from_c(&*cfg)
    };

    let layout = crate::pipeline::compute_layout_config(html, &config);
    let json = layout.to_json();

    match CString::new(json) {
        Ok(cs) => {
            *out_json_ptr = cs.into_raw();
            0
        }
        Err(_) => {
            set_last_error("JSON contained null byte");
            3
        }
    }
}

/// Render a PDF from a layout config JSON string.
///
/// This allows pre-computing the layout and rendering separately.
#[no_mangle]
pub unsafe extern "C" fn rpdf_render_from_layout(
    json_ptr: *const c_char,
    out_buf: *mut *mut u8,
    out_len: *mut u32,
) -> c_int {
    if json_ptr.is_null() || out_buf.is_null() || out_len.is_null() {
        set_last_error("Null pointer argument");
        return 1;
    }

    let json_cstr = CStr::from_ptr(json_ptr);
    let json = match json_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8 in JSON: {e}"));
            return 2;
        }
    };

    let layout_config = match crate::layout_config::LayoutConfig::from_json(json) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(&format!("Invalid layout JSON: {e}"));
            return 3;
        }
    };

    match crate::render::render_pdf(&layout_config) {
        Ok(pdf_bytes) => {
            let len = pdf_bytes.len() as u32;
            let buf = pdf_bytes.into_boxed_slice();
            let raw = Box::into_raw(buf) as *mut u8;
            *out_buf = raw;
            *out_len = len;
            0
        }
        Err(e) => {
            set_last_error(&e);
            4
        }
    }
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

/// Free a PDF buffer returned by `rpdf_generate_pdf`.
///
/// # Safety
/// `buf` must have been returned by a previous `rpdf_generate_pdf` (or similar)
/// call, and `len` must be the corresponding length.
#[no_mangle]
pub unsafe extern "C" fn rpdf_free_buffer(buf: *mut u8, len: u32) {
    if !buf.is_null() {
        let _ = Box::from_raw(slice::from_raw_parts_mut(buf, len as usize));
    }
}

/// Free a string returned by `rpdf_last_error` or layout config JSON.
///
/// # Safety
/// `s` must have been returned by Rust's `CString::into_raw`.
#[no_mangle]
pub unsafe extern "C" fn rpdf_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Retrieve the last error message. Returns a null-terminated string.
///
/// The returned pointer is valid until the next `rpdf_*` call on the same
/// thread. The caller should **not** free this pointer – it is managed
/// internally.
///
/// Returns null if no error has occurred.
#[no_mangle]
pub extern "C" fn rpdf_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        let borrow = e.borrow();
        match borrow.as_ref() {
            Some(cs) => cs.as_ptr(),
            None => ptr::null(),
        }
    })
}

/// Return the library version as a null-terminated string.
/// The caller must **not** free this pointer.
#[no_mangle]
pub extern "C" fn rpdf_version() -> *const c_char {
    // Safe: the string is static
    b"0.1.0\0".as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_generate_pdf() {
        let html = b"<h1>Hello FFI</h1>";
        let mut out_buf: *mut u8 = ptr::null_mut();
        let mut out_len: u32 = 0;

        let rc = unsafe {
            rpdf_generate_pdf(
                html.as_ptr(),
                html.len() as u32,
                &mut out_buf,
                &mut out_len,
            )
        };

        assert_eq!(rc, 0, "Expected success");
        assert!(!out_buf.is_null());
        assert!(out_len > 100);

        // Verify PDF header
        let bytes = unsafe { slice::from_raw_parts(out_buf, out_len as usize) };
        assert_eq!(&bytes[0..5], b"%PDF-");

        // Free
        unsafe { rpdf_free_buffer(out_buf, out_len) };
    }

    #[test]
    fn ffi_compute_layout() {
        let html = b"<p>Layout test</p>";
        let mut json_ptr: *mut c_char = ptr::null_mut();

        let rc = unsafe {
            rpdf_compute_layout(html.as_ptr(), html.len() as u32, &mut json_ptr)
        };

        assert_eq!(rc, 0);
        assert!(!json_ptr.is_null());

        let json = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap();
        assert!(json.contains("pages"));
        assert!(json.contains("page_width_pt"));

        unsafe { rpdf_free_string(json_ptr) };
    }

    #[test]
    fn ffi_null_input() {
        let mut out_buf: *mut u8 = ptr::null_mut();
        let mut out_len: u32 = 0;

        let rc = unsafe {
            rpdf_generate_pdf(ptr::null(), 0, &mut out_buf, &mut out_len)
        };

        assert_ne!(rc, 0, "Should fail on null input");
    }

    #[test]
    fn ffi_version() {
        let v = rpdf_version();
        let version = unsafe { CStr::from_ptr(v) }.to_str().unwrap();
        assert_eq!(version, "0.1.0");
    }

    #[test]
    fn ffi_generate_pdf_ex_null_config_uses_defaults() {
        let html = b"<h1>Hello ex</h1>";
        let mut out_buf: *mut u8 = ptr::null_mut();
        let mut out_len: u32 = 0;

        let rc = unsafe {
            rpdf_generate_pdf_ex(
                html.as_ptr(),
                html.len() as u32,
                ptr::null(),
                &mut out_buf,
                &mut out_len,
            )
        };

        assert_eq!(rc, 0);
        assert!(!out_buf.is_null());
        let bytes = unsafe { slice::from_raw_parts(out_buf, out_len as usize) };
        assert_eq!(&bytes[0..5], b"%PDF-");
        unsafe { rpdf_free_buffer(out_buf, out_len) };
    }

    #[test]
    fn ffi_generate_pdf_ex_custom_config() {
        use std::ffi::CString;

        let html = b"<h1>Landscape Test</h1>";
        let title = CString::new("My Custom Title").unwrap();
        let cfg = RpdfPipelineConfig {
            title: title.as_ptr(),
            page_width: 0.0,   // default
            page_height: 0.0,  // default
            page_margin: 20.0,
            orientation: RpdfPageOrientation::Landscape,
        };

        let mut out_buf: *mut u8 = ptr::null_mut();
        let mut out_len: u32 = 0;

        let rc = unsafe {
            rpdf_generate_pdf_ex(
                html.as_ptr(),
                html.len() as u32,
                &cfg,
                &mut out_buf,
                &mut out_len,
            )
        };

        assert_eq!(rc, 0, "Expected success");
        assert!(!out_buf.is_null());
        let bytes = unsafe { slice::from_raw_parts(out_buf, out_len as usize) };
        assert_eq!(&bytes[0..5], b"%PDF-");
        unsafe { rpdf_free_buffer(out_buf, out_len) };
    }

    #[test]
    fn ffi_compute_layout_ex_landscape() {
        use std::ffi::CString;

        let html = b"<p>Landscape layout</p>";
        let cfg = RpdfPipelineConfig {
            title: ptr::null(),
            page_width: 0.0,
            page_height: 0.0,
            page_margin: 0.0,
            orientation: RpdfPageOrientation::Landscape,
        };
        let mut json_ptr: *mut c_char = ptr::null_mut();

        let rc = unsafe {
            rpdf_compute_layout_ex(
                html.as_ptr(),
                html.len() as u32,
                &cfg,
                &mut json_ptr,
            )
        };

        assert_eq!(rc, 0);
        assert!(!json_ptr.is_null());
        let json = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap();
        // In landscape the effective width = A4 height (841.89)
        assert!(json.contains("841"), "Expected landscape width in JSON: {json}");
        unsafe { rpdf_free_string(json_ptr) };
    }
}
