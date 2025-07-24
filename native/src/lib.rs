use std::{ffi, ptr::slice_from_raw_parts_mut, sync::mpsc, thread, time::Duration};

use cpal::{
    InputCallbackInfo,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::utils::{Context, VirgilResult, deserialize, serialize};

mod messages;
mod utils;

// FIXME: Null checks!!

/// The expected sample rate and buffer size.
const EXPECTED_SAMPLE_RATE: usize = 16_000;

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
    // Decode model path and wake words
    let model_path: String = deserialize(model_path, model_path_len)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    let wake_words: Vec<String> = deserialize(wake_words, wake_words_len)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Encode context
    let transcript_capacity = 1024;
    let ctx = Context {
        model_path,
        wake_words,
        transcript: String::with_capacity(transcript_capacity),
    };
    serialize(ctx, ctx_len_out)
        .map_err(|e| eprintln!("{e}"))
        .unwrap()
}

/// Listens for wake words.
#[unsafe(no_mangle)]
pub fn listen_for_wake_words(ctx: *mut ffi::c_void, ctx_len: usize, miliseconds: usize) -> bool {
    // Decode context
    let ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();
    let wake_words = ctx.wake_words.clone();

    // Initialize `Whisper` model
    let model_ctx =
        WhisperContext::new_with_params(&ctx.model_path, WhisperContextParameters::default())
            .map_err(|e| eprintln!("{e}"))
            .unwrap();
    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let mut model = model_ctx
        .create_state()
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Initialize microphone
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .ok_or_else(|| "Default input device not found".to_string())
        .unwrap();
    let config = input_device.default_input_config().unwrap().config();

    // Setup channels for communication w/ threads
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let (detect_tx, detect_rx) = mpsc::channel::<bool>();

    // Initialize input stream (microphone)
    let input_callback = move |data: &[f32], _: &InputCallbackInfo| {
        tx.send(data.into()).unwrap();
    };
    let input_stream = input_device
        .build_input_stream(&config, input_callback, |err| eprintln!("{err}"), None)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Start stream and process data in thread.
    input_stream.play().map_err(|e| eprintln!("{e}")).unwrap();
    thread::spawn(move || {
        let mut accumulated_data = Vec::<f32>::with_capacity(EXPECTED_SAMPLE_RATE);
        while let Ok(data) = rx.recv() {
            accumulated_data.extend_from_slice(&data);
        }

        // Check for wake words
        let wake_word_detected =
            detect_wake_words(&mut model, params.clone(), &wake_words, &accumulated_data)
                .map_err(|e| eprintln!("{e}"))
                .unwrap();
        detect_tx
            .send(wake_word_detected)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();
    });

    // Listen for specified duration
    std::thread::sleep(Duration::from_millis(miliseconds as u64));

    // Return
    let mut wake_word_detected = false;
    while let Ok(detected) = detect_rx.recv() {
        if detected {
            wake_word_detected = true;
            break;
        }
    }
    wake_word_detected
}

/// Listens for commands.
///
/// This should be called after [listen_for_wake_word] to actively listen for a longer duration.
#[unsafe(no_mangle)]
pub fn listen_for_commands(
    ctx: *mut ffi::c_void,
    ctx_len: usize,
    miliseconds: usize,
    ctx_len_out: *mut usize,
) -> *mut ffi::c_void {
    // Decode context
    let mut ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Initialize `Whisper` model
    let model_ctx =
        WhisperContext::new_with_params(&ctx.model_path, WhisperContextParameters::default())
            .map_err(|e| eprintln!("{e}"))
            .unwrap();
    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let mut model = model_ctx
        .create_state()
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Initialize microphone
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .ok_or_else(|| "Default input device not found".to_string())
        .unwrap();
    let config = input_device.default_input_config().unwrap().config();

    // Setup channels for communication w/ threads
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let (transcript_tx, transcript_rx) = mpsc::channel::<String>();

    // Initialize input stream (microphone)
    let input_callback = move |data: &[f32], _: &InputCallbackInfo| {
        tx.send(data.into()).unwrap();
    };
    let input_stream = input_device
        .build_input_stream(&config, input_callback, |err| eprintln!("{err}"), None)
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

    // Start stream and process data in thread.
    input_stream.play().map_err(|e| eprintln!("{e}")).unwrap();
    thread::spawn(move || {
        let mut accumulated_data = Vec::<f32>::with_capacity(EXPECTED_SAMPLE_RATE);
        while let Ok(data) = rx.recv() {
            accumulated_data.extend_from_slice(&data);
        }

        // Transcribe accumulated data
        let transcript = transcribe(&mut model, params, &accumulated_data)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();
        transcript_tx
            .send(transcript)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();
    });

    // Listen for specified duration
    std::thread::sleep(Duration::from_millis(miliseconds as u64));

    // Return updated context
    let mut transcript = String::with_capacity(1024);
    while let Ok(text) = transcript_rx.recv() {
        transcript.push_str(&text);
    }
    ctx.transcript = transcript;
    serialize(ctx, ctx_len_out)
        .map_err(|e| eprintln!("{e}"))
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

#[cfg(test)]
mod tests {
    use crate::messages::Message;

    use super::*;

    #[test]
    fn wake_word() -> VirgilResult<()> {
        let out_len: usize = 0;
        let len_out = (&out_len as *const usize).cast_mut();

        let model_path: String = "test_assets/ggml-tiny.bin".into();
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
        let detected = listen_for_wake_words(ctx, ctx_len, 1000);
        dbg!(detected);

        Ok(())
    }
}
