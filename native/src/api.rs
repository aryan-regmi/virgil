use std::{ffi, ptr::slice_from_raw_parts_mut};

use tracing::{Level, error, span, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use whisper_rs::install_logging_hooks;

use crate::utils::{Context, deserialize, serialize};

/// Sets up logging for the library.
#[unsafe(no_mangle)]
pub fn setup_logs() {
    // Suppress logs from `whisper.cpp`.
    install_logging_hooks();

    // Filter specific crates by log levels
    let filter = filter::Targets::new()
        .with_target("native", Level::TRACE)
        .with_target("whisper-rs", Level::ERROR);
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_line_number(true)
                .with_target(true),
        )
        .init()
}

/// Frees the memory allocated by Rust.
#[unsafe(no_mangle)]
pub fn free_rust_ptr(ptr: *mut ffi::c_void, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let ptr: *mut u8 = ptr.cast();
        let _ = Box::from_raw(slice_from_raw_parts_mut(ptr, len));
    }
}

/// Initalizes the application context.
#[unsafe(no_mangle)]
pub fn init_context(
    model_path: *mut ffi::c_void,
    model_path_len: usize,
    wake_words: *mut ffi::c_void,
    wake_words_len: usize,
    ctx_len_out: *mut usize,
) -> *mut ffi::c_void {
    let span = span!(Level::TRACE, "init_context");
    let _enter = span.enter();

    // Decode model path and wake words
    let model_path: String = deserialize(model_path, model_path_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    trace!("Model path decoded: {model_path}");

    let wake_words: Vec<String> = deserialize(wake_words, wake_words_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    trace!("Wake words decoded: {wake_words:?}");

    // Encode context
    let transcript_capacity = 1024;
    let ctx = Context {
        model_path,
        wake_words,
        transcript: String::with_capacity(transcript_capacity),
    };
    let encoded_ctx = serialize(ctx, ctx_len_out)
        .map_err(|e| error!("{e}"))
        .unwrap();
    trace!("Context encoded");
    encoded_ctx
}
