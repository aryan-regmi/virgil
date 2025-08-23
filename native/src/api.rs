use std::{ffi, ptr::slice_from_raw_parts_mut, sync::LazyLock, thread};

use kalosm::sound::*;
use kalosm_common::Cache;
use tokio::{runtime::Runtime, sync::Mutex};
use tracing::{Level, debug, error, info, span};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    port::{DartPort, send_text_to_dart, set_dart_port},
    utils::{Context, deserialize, serialize},
};

pub static RUN: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

pub static LOGS_SET: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// Sets up logging for the library.
#[unsafe(no_mangle)]
pub fn setup_logs(level: usize) {
    let log_level = match level {
        0 => Level::TRACE,
        1 => Level::DEBUG,
        2 => Level::INFO,
        3 => Level::WARN,
        4 => Level::ERROR,
        _ => Level::TRACE,
    };

    let mut logs_set = LOGS_SET.blocking_lock();
    if *logs_set == false {
        // Suppress logs from `whisper.cpp`.
        // install_logging_hooks();

        // Filter specific crates by log levels
        let filter = filter::Targets::new()
            .with_target("native", log_level)
            .with_target("whisper-rs", Level::ERROR);
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_line_number(true)
                    .with_target(true),
            )
            .init();

        *logs_set = true;
    }
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
    debug!("Model path decoded: {model_path}");

    let wake_words: Vec<String> = deserialize(wake_words, wake_words_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Wake words decoded: {wake_words:?}");

    // Encode context
    let ctx = Context {
        model_path,
        wake_words,
    };
    let encoded_ctx = serialize(ctx, ctx_len_out)
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Context encoded");

    encoded_ctx
}

/// Initalizes the Dart Native API.
#[unsafe(no_mangle)]
pub fn init_dart_api(data: *mut std::ffi::c_void) -> isize {
    unsafe { dart_sys::Dart_InitializeApiDL(data) }
}

/// Initalizes the Dart port for communication.
#[unsafe(no_mangle)]
pub fn init_dart_port(port: DartPort) {
    set_dart_port(port);
}

/// Turns microphone input into text.
#[unsafe(no_mangle)]
pub fn transcribe_speech(ctx: *mut ffi::c_void, ctx_len: usize) {
    let span = span!(Level::TRACE, "transcribe_speech");
    let _enter = span.enter();

    // Init tokio runtime
    let rt = Runtime::new().unwrap();

    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Context decoded");

    // Transcribe speech
    rt.block_on(async move {});

    thread::spawn(move || {
        rt.block_on(async move {
            tokio::select! {
                _ = tokio::spawn(async move {
                    process(ctx).await;
                }) => {},
                _ = futures::future::pending::<()>() => {
                    if *RUN.lock().await == false {
                        return;
                    }
                },
            }
        });
    });
}

/// Stops the microphone.
#[unsafe(no_mangle)]
pub fn stop_mic() {
    let span = span!(Level::TRACE, "stop_mic");
    let _enter = span.enter();
    info!("Mic stopped!");
    *RUN.blocking_lock() = false;
}

async fn process(ctx: Context) {
    // Setup model
    let model = Whisper::builder()
        .with_cache(Cache::new(ctx.model_path.into()))
        .build()
        .await
        .map_err(|e| error!("Unable to build model: {e}"))
        .unwrap();

    // Setup mic stream
    let mic = MicInput::default();
    let stream = mic.stream();

    // Transcribe audio
    let transcript = stream.transcribe(model);
    let transcript = transcript
        .map(|s| s.text().into())
        .collect::<Vec<String>>()
        .await
        .join(" ");

    // Send to Dart
    info!("Transcript: {transcript:?}");
    send_text_to_dart(transcript)
        .map_err(|e| error!("Unable to send transcript to Dart: {e}"))
        .unwrap_or_else(|_| return);
}
