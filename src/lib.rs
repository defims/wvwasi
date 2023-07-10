//! Re-exported wry APIs
//!
//! This module re-export [wry] APIs for user to create webview. To learn more about
//! how to use wry, please see [its documentation](https://crates.io/crates/wry).
//!
//! [wry]: https://crates.io/crates/wry

pub use wvwasi_wry::*;

pub mod webview;
pub mod wasi;