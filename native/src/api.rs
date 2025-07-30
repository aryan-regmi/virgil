use std::{ffi, ptr::slice_from_raw_parts_mut, thread, time::Duration};

use cpal::traits::StreamTrait;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self},
};
use tracing::{Level, Span, debug, error, info, span};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use whisper_rs::{FullParams, SamplingStrategy, WhisperState, install_logging_hooks};

use crate::{
    port::{DartPort, send_text_to_dart, set_dart_port},
    utils::{
        Context, EXPECTED_SAMPLE_RATE, SendStream, VirgilResult, deserialize, detect_wake_words,
        init_microphone, init_model, serialize, transcribe,
    },
};

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

    // Suppress logs from `whisper.cpp`.
    install_logging_hooks();

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
    let transcript_capacity = 1024;
    let ctx = Context {
        model_path,
        wake_words,
        transcript: String::with_capacity(transcript_capacity),
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

// FIXME: Do this in a background thread?
//
/// Turns microphone input into text.
#[unsafe(no_mangle)]
pub fn transcribe_speech(ctx: *mut ffi::c_void, ctx_len: usize, listen_duration_ms: usize) {
    let span = span!(Level::TRACE, "transcribe_speech");
    let _enter = span.enter();

    let listen_duration_ms = listen_duration_ms as u64;

    // Init tokio runtime
    let rt = Runtime::new().unwrap();

    // Setup channels for communication
    let (input_audio_tx, input_audio_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);

    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Context decoded");

    // Init `Whisper` model
    let model = init_model(&ctx.model_path)
        .map_err(|e| error!("{e}"))
        .unwrap();

    // Initalize microphone
    let mic = SendStream(
        init_microphone(input_audio_tx.clone())
            .map_err(|e| error!("{e}"))
            .unwrap(),
    );

    // Listen to the microphone for the specified amount of time
    rt.spawn(async move {
        let span = span!(Level::TRACE, "listener");
        let _enter = span.enter();

        mic.0
            .play()
            .map_err(|e| error!("Failed to start listening to mic: {e}"))
            .unwrap();
        info!("Listening to microphone...");

        loop {
            tokio::time::sleep(Duration::from_millis(listen_duration_ms)).await;
        }
    });

    info!("Processing microphone input...");
    let parent_span = span.clone();
    thread::spawn(move || {
        rt.block_on(async move {
            tokio::spawn(process(
                ctx,
                model,
                input_audio_rx,
                listen_duration_ms,
                parent_span,
            ));

            // Keep tokio runtime alive
            futures::future::pending::<()>().await
        });
    });
}

async fn process(
    ctx: Context,
    mut model: WhisperState,
    mut input_audio_rx: mpsc::Receiver<Vec<f32>>,
    listen_duration_ms: u64,
    parent_span: Span,
) {
    let span = span!(parent: &parent_span, Level::TRACE, "process");
    let _enter = span.enter();

    let desired_num_samples = (listen_duration_ms as usize / 1000) * EXPECTED_SAMPLE_RATE + 200;
    let mut accumulated_audio = Vec::with_capacity(desired_num_samples);
    loop {
        while let Ok(audio_data) = input_audio_rx.try_recv() {
            let accumulated_samples = accumulated_audio.len();
            let samples_to_add = audio_data.len();
            let num_samples = accumulated_samples + samples_to_add;

            // Accumulate audio data until desired length is reached
            if num_samples < desired_num_samples {
                accumulated_audio.extend_from_slice(&audio_data);
                continue;
            }

            // If more than desired samples, send exact amount then restart accumulation
            if num_samples >= desired_num_samples {
                let extra = num_samples - desired_num_samples;
                let end_idx = samples_to_add - extra;

                // Send desired number of samples
                accumulated_audio.extend_from_slice(&audio_data[0..end_idx]);
                debug!("Accumulated {} samples", accumulated_audio.len());

                // FIXME: Handle multiple channels

                // Transcribe data
                let text = transcribe_audio_data(&mut model, &accumulated_audio, &ctx.wake_words)
                    .map_err(|e| error!("Unable to process audio: {e}"))
                    .unwrap();

                // Send transcript to Dart
                send_text_to_dart(text)
                    .map_err(|e| error!("Unable to send text to Dart: {e}"))
                    .unwrap();
                debug!("Transcript updated");

                // Reset accumulated data and fill with remaining/overflowing samples
                debug!("Accumulated data reset");
                accumulated_audio.clear();
                accumulated_audio.extend_from_slice(&audio_data[end_idx..]);

                continue;
            }
        }

        std::thread::sleep(Duration::from_millis(listen_duration_ms));
    }
}

/// Processes the audio data by transcibing audio data if wake words are detected.
fn transcribe_audio_data(
    model: &mut WhisperState,
    audio_data: &[f32],
    wake_words: &Vec<String>,
) -> VirgilResult<String> {
    let span = span!(Level::TRACE, "transcribe_audio_data");
    let _enter = span.enter();

    debug!("Processing {} samples", audio_data.len());

    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let wake_word_detected = detect_wake_words(model, params.clone(), audio_data, wake_words)?;
    if wake_word_detected {
        info!("Wake word detected");
    }
    // FIXME: Move into wake_word_detected check
    let text = transcribe(model, params, audio_data)?;

    Ok(text)
}
