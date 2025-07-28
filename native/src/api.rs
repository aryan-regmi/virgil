use std::{
    ffi,
    ptr::slice_from_raw_parts_mut,
    time::{Duration, Instant},
};

use cpal::traits::StreamTrait;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Receiver, Sender},
    time::sleep,
};
use tracing::{Level, debug, error, span, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use whisper_rs::{FullParams, SamplingStrategy, WhisperState, install_logging_hooks};

use crate::utils::{
    Context, EXPECTED_SAMPLE_RATE, SendStream, VirgilResult, accumulate_audio_data, deserialize,
    detect_wake_words, init_microphone, init_model, serialize, transcribe,
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
        // .with(filter)
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

/// Listens continuously to the microphone and transcribes the input if a wake word was detected.
#[unsafe(no_mangle)]
pub fn transcribe_speech(
    ctx: *mut ffi::c_void,
    ctx_len: usize,
    listen_duration_ms: usize,
    mut _ctx_out: *mut ffi::c_void,
    ctx_len_out: *mut usize,
) -> ! {
    let span = span!(Level::TRACE, "transcribe_speech");
    let _enter = span.enter();

    // Init tokio runtime
    let rt = Runtime::new().map_err(|e| error!("{e}")).unwrap();
    let _rt_guard = rt.enter();

    // Setup channels for communication
    let (input_audio_tx, input_audio_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);

    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();

    // Init `Whisper` model
    let mut model = init_model(&ctx.model_path)
        .map_err(|e| error!("{e}"))
        .unwrap();
    trace!("Model initalized");

    // Initalize microphone
    let mic = SendStream(
        init_microphone(input_audio_tx.clone())
            .map_err(|e| error!("{e}"))
            .unwrap(),
    );

    // Start listening
    let _h = rt.spawn(async move {
        listen_to_mic(&mic, listen_duration_ms as u64)
            .await
            .map_err(|e| error!("{e}"))
            .unwrap();
    });

    // Accumulate audio data
    let (accumaltor_tx, accumaltor_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);
    let _h = rt.spawn(async move {
        accumulate_audio_data(accumaltor_tx, input_audio_rx, listen_duration_ms)
    });

    // Process the data
    let (text_tx, mut text_rx) = mpsc::channel::<String>(2048);
    let wake_words = ctx.wake_words.clone();
    rt.spawn_blocking(async move || {
        process_audio_data(&mut model, &wake_words, accumaltor_rx, text_tx)
            .await
            .map_err(|e| error!("{e}"))
            .unwrap();
    });

    // Update the context with the transcribed text
    let start_time = Instant::now();
    let timeout = Duration::from_millis(listen_duration_ms as u64);
    loop {
        let mut updated_ctx = Context {
            model_path: ctx.model_path.clone(),
            wake_words: ctx.wake_words.clone(),
            transcript: String::new(),
        };

        if start_time.elapsed() > timeout {
            _ctx_out = serialize(updated_ctx, ctx_len_out)
                .map_err(|e| error!("{e}"))
                .unwrap();
            continue;
        }

        while let Ok(text) = text_rx.try_recv() {
            // FIXME: Add `max_transcript_len` param and make sure `text` is less than that
            updated_ctx.transcript = text;
        }
    }
}

/// Continuously listens to the microphone for the specified duration.
async fn listen_to_mic(mic: &SendStream, listen_duration_ms: u64) -> VirgilResult<()> {
    let span = span!(Level::TRACE, "listen_to_mic");
    let _enter = span.enter();
    trace!("Listening for input...");

    // Start listening
    mic.0.play()?;

    // Keep the stream alive
    loop {
        sleep(Duration::from_millis(listen_duration_ms)).await;
    }
}

async fn process_audio_data(
    model: &mut WhisperState,
    wake_words: &Vec<String>,
    mut accumaltor_rx: Receiver<Vec<f32>>,
    text_tx: Sender<String>,
) -> VirgilResult<()> {
    let span = span!(Level::TRACE, "process_audio_data");
    let _enter = span.enter();

    while let Some(audio_data) = &accumaltor_rx.recv().await {
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        trace!("Detecting wake words");
        let wake_word_detected = detect_wake_words(model, params.clone(), audio_data, wake_words)?;

        if wake_word_detected {
            trace!("Wake word detected");

            // TODO: Process commands
            let text = transcribe(model, params, audio_data)?;
            text_tx.send(text).await?;
        }
    }

    Ok(())
}
