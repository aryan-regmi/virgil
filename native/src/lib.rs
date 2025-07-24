use std::{ffi, ptr::slice_from_raw_parts_mut, time::Duration};

use cpal::{
    InputCallbackInfo, SampleRate,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use tokio::time::Instant;
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

// FIXME: Change to be actual listener!
#[unsafe(no_mangle)]
pub async fn test_listen(ctx: *mut ffi::c_void, ctx_len: usize, miliseconds: usize) {
    let (audio_data_tx, mut audio_data_rx) =
        tokio::sync::mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);

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

    // Spawn task to listen to microphone and capture audio data
    tokio::spawn(async move {
        // Initialize microphone
        let host = cpal::default_host();
        let input_device = host
            .default_input_device()
            .ok_or_else(|| "Default input device not found".to_string())
            .unwrap();
        let config = input_device
            .supported_input_configs()
            .unwrap()
            .next()
            .unwrap()
            .with_sample_rate(SampleRate(EXPECTED_SAMPLE_RATE as u32))
            .config();

        // Initialize input stream (microphone)
        let stream = input_device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &InputCallbackInfo| {
                    let num_channels = config.channels as usize;
                    if num_channels > 1 {
                        // TODO: Split audio channels!
                        let channels = data.chunks_exact(num_channels);
                        for channel_audio in channels {
                            audio_data_tx.try_send(channel_audio.into()).unwrap()
                        }
                    } else {
                        audio_data_tx.try_send(data.into()).unwrap()
                    }
                },
                |err| eprintln!("{err}"),
                None,
            )
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        // Start the stream
        stream.play().map_err(|e| eprintln!("{e}")).unwrap();

        // Keep the stream alive
        loop {
            tokio::time::sleep(Duration::from_millis(miliseconds as u64)).await
        }
    });

    // TODO: Merge channels?
    //
    // Accumulate audio data until sample is large enough
    let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);
    tokio::spawn(async move {
        let mut accumulated_data = Vec::with_capacity(EXPECTED_SAMPLE_RATE);
        while let Some(data) = audio_data_rx.recv().await {
            accumulated_data.extend_from_slice(&data);

            if accumulated_data.len() >= EXPECTED_SAMPLE_RATE {
                audio_tx.send(accumulated_data.clone()).await.unwrap();
                accumulated_data.clear();
            }
        }
    });

    // Process data
    let start_time = Instant::now();
    let timeout = Duration::from_millis(miliseconds as u64);
    let mut transcript = String::with_capacity(1024);
    loop {
        if start_time.elapsed() > timeout {
            println!("Timed out");
            break;
        }

        if let Some(audio_data) = audio_rx.recv().await {
            // Check for wake words
            let wake_word_detected =
                detect_wake_words(&mut model, params.clone(), &wake_words, &audio_data)
                    .map_err(|e| eprintln!("{e}"))
                    .unwrap();

            if wake_word_detected {
                println!("Wake word detected!");
            }

            // Transcript
            let text = transcribe(&mut model, params.clone(), &audio_data)
                .map_err(|e| eprintln!("{e}"))
                .unwrap();
            transcript.push_str(text.trim());
        }
    }
    dbg!(transcript);
}

#[cfg(test)]
mod tests {
    use crate::messages::Message;

    use super::*;

    fn get_context() -> VirgilResult<(*mut ffi::c_void, usize)> {
        supress_whisper_logs();
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
        Ok((ctx, ctx_len))
    }

    #[tokio::test]
    async fn test_listenr() -> VirgilResult<()> {
        let (ctx, ctx_len) = get_context()?;
        test_listen(ctx, ctx_len, 1000).await;
        Ok(())
    }
}
