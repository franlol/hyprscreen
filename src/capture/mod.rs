//! Capture pipeline for screenshots and recordings.

use chrono::Local;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub mod record;
pub mod screenshot;

pub fn generated_filename(extension: &str) -> String {
    let config = crate::config::get();
    let formatted = catch_unwind(AssertUnwindSafe(|| {
        Local::now().format(&config.timestamp_format).to_string()
    }))
    .ok()
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| Local::now().format("%H%M%S%d%m%Y").to_string());

    format!("{}-{}.{}", config.filename_prefix, formatted, extension)
}
