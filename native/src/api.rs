use std::{
    ffi,
    ptr::slice_from_raw_parts_mut,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use cpal::{Stream, traits::StreamTrait};
use tokio::{runtime::Runtime, sync::mpsc, time::sleep};
use tracing::{Level, error, span, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use whisper_rs::{FullParams, SamplingStrategy, WhisperState, install_logging_hooks};

use crate::utils::{
    Context, EXPECTED_SAMPLE_RATE, accumulate_audio_data, deserialize, detect_wake_words,
    init_microphone, init_model, serialize,
};

struct Model {
    state: Option<WhisperState>,
}
const MODEL: LazyLock<Mutex<Model>> = LazyLock::new(|| Mutex::new(Model { state: None }));

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
        wake_words,
        transcript: String::with_capacity(transcript_capacity),
    };
    let encoded_ctx = serialize(ctx, ctx_len_out)
        .map_err(|e| error!("{e}"))
        .unwrap();
    trace!("Context encoded");

    // Update model
    let model = MODEL;
    let mut model = model.lock().map_err(|e| error!("{e}")).unwrap();
    model.state = Some(init_model(&model_path).map_err(|e| error!("{e}")).unwrap());

    encoded_ctx
}

#[unsafe(no_mangle)]
pub fn transcribe_speech(
    ctx: *mut ffi::c_void,
    ctx_len: usize,
    timeout_ms: usize,
    ctx_len_out: *mut usize,
) -> *mut ffi::c_void {
    let span = span!(Level::TRACE, "transcribe_speech");
    let _enter = span.enter();

    // Init tokio runtime
    let rt = Runtime::new().map_err(|e| error!("{e}")).unwrap();

    // Setup channels for communication
    let (input_audio_tx, input_audio_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);

    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();

    // Start listening and transcribe the input
    // Initalize microphone
    let mic = init_microphone(input_audio_tx)
        .map_err(|e| error!("{e}"))
        .unwrap();

    // Start listening
    rt.spawn(async move {
        listen_to_mic(&mic, timeout_ms as u64).await;
    });

    // Accumulate audio data
    let (accumaltor_tx, mut accumaltor_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);
    rt.spawn(async move { accumulate_audio_data(accumaltor_tx, input_audio_rx, timeout_ms) });

    // Process the data
    rt.spawn_blocking(async move || {
        let model = MODEL;
        let mut model = model.lock().unwrap();
        if let Some(model) = model.state.as_mut() {
            while let Some(audio_data) = &accumaltor_rx.recv().await {
                trace!("Detecting wake words");
                let wake_word_detected = detect_wake_words(
                    model,
                    FullParams::new(SamplingStrategy::Greedy { best_of: 1 }),
                    audio_data,
                    &ctx.wake_words,
                );
            }
        }
    });

    todo!()
}

async fn listen_to_mic(mic: &Stream, timeout_ms: u64) {
    // Start listening
    mic.play().map_err(|e| error!("{e}")).unwrap();

    // Keep the stream alive
    loop {
        sleep(Duration::from_millis(timeout_ms));
    }
}
