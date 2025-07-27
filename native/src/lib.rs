use std::{
    ffi,
    ptr::slice_from_raw_parts_mut,
    thread::{self, sleep},
    time::Duration,
};

use bincode::encode_to_vec;
use cpal::{
    InputCallbackInfo, SampleRate,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::sync::mpsc;
use std::time::Instant;
use tracing::{Level, debug, error, span, trace};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    install_logging_hooks,
};

use crate::utils::{Context, VirgilResult, deserialize, serialize};

mod messages;
mod utils;

// FIXME: Null checks!!

/// The expected sample rate and buffer size.
const EXPECTED_SAMPLE_RATE: usize = 16_000;

// FIXME: Setup tracing subscriber!

/// Suppress logs from `whisper.cpp`.
#[unsafe(no_mangle)]
pub fn supress_whisper_logs() {
    install_logging_hooks();
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

// TODO: Have lazy static for the actual model?
//  - Keeps from creating new state every call
//
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

/// Listens to microphone input and transcribes it to text.
#[unsafe(no_mangle)]
pub fn transcribe_speech(
    ctx: *mut ffi::c_void,
    ctx_len: usize,
    timeout_ms: usize,
    ctx_len_out: *mut usize,
) -> *mut ffi::c_void {
    const LOG_LEVEL: Level = Level::TRACE;
    let span = span!(LOG_LEVEL, "init_context");
    let _enter = span.enter();

    // Setup channels for communication
    let (audio_data_tx, audio_data_rx) = mpsc::channel::<Vec<f32>>();

    // Decode context
    let mut ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| error!("{e}"))
        .unwrap();
    let wake_words = ctx.wake_words.clone();
    debug!("Context decoded");

    // Initialize `Whisper` model
    let model_ctx =
        WhisperContext::new_with_params(&ctx.model_path, WhisperContextParameters::default())
            .map_err(|e| error!("{e}"))
            .unwrap();
    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 2 });
    let mut model = model_ctx.create_state().map_err(|e| error!("{e}")).unwrap();
    debug!("Model initalized");

    // Spawn task to listen to microphone and capture audio data
    // thread::spawn(move || listen_for_duration(audio_data_tx, timeout_ms as u64));
    listen_for_duration(audio_data_tx, timeout_ms as u64);
    debug!("Listening...");

    // Accumulate audio data until sample is large enough
    let accumulator = accumulate_audio_data(audio_data_rx, EXPECTED_SAMPLE_RATE);
    debug!("Audio data accumulated");

    // Process data
    // Transcribe audio data if wake words detected
    let start_time = Instant::now();
    let timeout = Duration::from_millis(timeout_ms as u64);
    let mut transcript = String::with_capacity(1024);
    loop {
        if start_time.elapsed() > timeout {
            break;
        }

        // FIXME: Collect the data, transcribe all together at the end (not in the loop!)
        while let Ok(audio_data) = &accumulator.recv() {
            debug!("Detecting wake words...");
            let wake_word_detected =
                detect_wake_words(&mut model, params.clone(), &wake_words, &audio_data)
                    .map_err(|e| error!("{e}"))
                    .unwrap();
            // if wake_word_detected {
            let text = transcribe(&mut model, params.clone(), audio_data)
                .map_err(|e| error!("{e}"))
                .unwrap();
            transcript.push_str(&text);
            // }
        }
    }

    ctx.transcript = transcript;
    serialize(ctx, ctx_len_out)
        .map_err(|e| error!("{e}"))
        .unwrap()
}

/// Checks for wake words in audio data.
fn detect_wake_words(
    model: &mut WhisperState,
    params: FullParams,
    wake_words: &Vec<String>,
    audio_data: &[f32],
) -> VirgilResult<bool> {
    let transcript = transcribe(model, params, audio_data)?.to_lowercase();

    for word in wake_words {
        if transcript.contains(&word.to_lowercase()) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Converts audio data to text.
fn transcribe(
    model: &mut WhisperState,
    params: FullParams,
    audio_data: &[f32],
) -> VirgilResult<String> {
    model.full(params, audio_data)?;
    let mut transcript = String::with_capacity(1026);
    let num_segments = model.full_n_segments().unwrap();
    for i in 0..num_segments {
        let segment = model.full_get_segment_text(i).unwrap();
        transcript.push_str(&segment);
    }
    Ok(transcript)
}

/// Initalizes the microphone and listens for the specified number of milliseconds.
fn listen_for_duration(sender: mpsc::Sender<Vec<f32>>, listen_duration_ms: u64) {
    // Initialize microphone
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .ok_or_else(|| error!("Default input device not found"))
        .unwrap();
    let config = input_device
        .supported_input_configs()
        .map_err(|e| error!("{e}"))
        .unwrap()
        .next()
        .unwrap()
        .with_sample_rate(SampleRate(EXPECTED_SAMPLE_RATE as u32))
        .config();
    debug!("Microphone initalized");

    // Initialize input stream (microphone)
    let stream = input_device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &InputCallbackInfo| {
                let num_channels = config.channels as usize;
                if num_channels > 1 {
                    // FIXME: Merge outputs in some way
                    //  - Either audio data or the final transcript
                    //
                    // Split audio channels and process them separately
                    let channels = data.chunks_exact(num_channels);
                    for channel_audio in channels {
                        if sender.send(channel_audio.into()).is_err() {
                            error!("Input stream has shut down");
                            break;
                        }
                    }
                } else {
                    if sender.send(data.into()).is_err() {
                        error!("Input stream has shut down");
                        return;
                    }
                }
            },
            |e| error!("{e}"),
            None,
        )
        .map_err(|e| error!("{e}"))
        .unwrap();
    debug!("Input stream initalized");

    // Start the stream
    stream.play().map_err(|e| error!("{e}")).unwrap();
    debug!("Stream started!");

    // Keep the stream alive
    loop {
        sleep(Duration::from_millis(listen_duration_ms));
        break;
    }
}

/// Accumulates audio data until there are `min_num_samples` audio samples.
fn accumulate_audio_data(
    receiver: mpsc::Receiver<Vec<f32>>,
    min_num_samples: usize,
) -> mpsc::Receiver<Vec<f32>> {
    let (audio_tx, audio_rx) = mpsc::channel::<Vec<f32>>();
    thread::spawn(move || {
        let mut accumulated_data = Vec::with_capacity(min_num_samples);
        while let Ok(data) = receiver.try_recv() {
            accumulated_data.extend_from_slice(&data);
        }
        if audio_tx.send(accumulated_data.clone()).is_err() {
            error!("Failed to send accumulated audio data");
        }
        accumulated_data.clear();
    });
    audio_rx
}

#[cfg(test)]
mod tests {
    use crate::messages::Message;

    use tracing::debug;
    use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

    use super::*;

    fn get_context() -> VirgilResult<(*mut ffi::c_void, usize)> {
        supress_whisper_logs();
        let filter = filter::Targets::new()
            .with_target("native", Level::TRACE)
            .with_target("whisper-rs", Level::ERROR);
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_line_number(true)
                    .with_thread_ids(true),
            )
            .with(filter)
            .init();

        let out_len: usize = 0;
        let len_out = (&out_len as *const usize).cast_mut();

        let model_path: String = "test_assets/ggml-tiny.en.bin".into();
        let model_path_len = model_path.byte_len();
        let model_path = serialize(model_path, len_out)?;

        let wake_words: Vec<String> = vec!["Wake".into(), "Test".into()];
        let wake_words_len = wake_words.byte_len();
        let wake_words = serialize(wake_words, len_out)?;

        let ctx = init_context(
            model_path,
            model_path_len,
            wake_words,
            wake_words_len,
            len_out,
        );
        let ctx_len = unsafe { *len_out };
        Ok((ctx, ctx_len))
    }

    #[test]
    fn test_listener() -> VirgilResult<()> {
        let tst: usize = 0;
        let (ctx, ctx_len) = get_context()?;
        let ctx_len_out = &tst as *const usize;
        // loop {
        let transcript = transcribe_speech(ctx, ctx_len, 3000, ctx_len_out.cast_mut());
        let ctx: Context = unsafe { deserialize(transcript, *ctx_len_out)? };
        if !ctx.transcript.is_empty() {
            debug!("transcription: {:?}", ctx.transcript);
        }
        // }
        Ok(())
    }
}
