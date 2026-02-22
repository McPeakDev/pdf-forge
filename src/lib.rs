//! # rpdf – Template-driven HTML → PDF pipeline
//!
//! This crate provides a complete pipeline for converting controlled HTML
//! templates into reproducible PDF documents. The pipeline stages are:
//!
//! 1. **Parse** – HTML string → DOM tree ([`dom`])
//! 2. **Style** – apply inline styles and Tailwind-like classes ([`style`])
//! 3. **Layout** – compute flexbox/grid layout with Taffy ([`layout`])
//! 4. **Paginate** – split into A4 pages ([`pagination`])
//! 5. **Render** – emit PDF bytes via printpdf ([`render`])
//!
//! A C-compatible FFI surface is exposed via the [`ffi`] module.

pub mod dom;
pub mod ffi;
pub mod fonts;
pub mod layout;
pub mod layout_config;
pub mod pagination;
pub mod pipeline;
pub mod render;
pub mod style;
pub mod templates;

// Re-exports for convenience
pub use pipeline::{generate_pdf, generate_pdf_from_html, PageOrientation};
