use std::{ffi, ptr::slice_from_raw_parts};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};
use cpal::{
    InputCallbackInfo, SampleRate, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait},
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{Level, error, info, span};
use whisper_rs::{
    FullParams, WhisperContext, WhisperContextParameters, WhisperState,
    convert_stereo_to_mono_audio,
};

use crate::messages::Message;

#[allow(dead_code)]
pub type VirgilResult<T> = Result<T, anyhow::Error>;

/// The context passed around for FFI functions.
#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
    pub transcript: String,
}

/// Serialize the given encodable value.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
pub fn serialize<T: Message>(
    value: T,
    value_len_out: *mut usize,
) -> VirgilResult<*mut ffi::c_void> {
    let mut bytes = vec![0; value.byte_len()];
    let written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )?;
    unsafe { *value_len_out = written };
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    Ok(response_ptr)
}

#[derive(Debug, Error)]
#[error("DeserializeError: {0}")]
pub struct DeserializeError(String);

/// Deserialize the value represented by the given pointer and length.
pub fn deserialize<T: Decode<()>>(ptr: *mut ffi::c_void, len: usize) -> VirgilResult<T> {
    let slice = unsafe {
        let ptr: *mut u8 = ptr.cast();
        slice_from_raw_parts(ptr, len)
            .as_ref()
            .ok_or_else(|| DeserializeError("Unable to get reference to raw slice".into()))?
    };

    let (decoded, _): (T, usize) =
        decode_from_slice(slice, bincode::config::standard().with_fixed_int_encoding())?;

    Ok(decoded)
}

#[allow(dead_code)]
/// The expected sample rate and buffer size.
pub const EXPECTED_SAMPLE_RATE: usize = 16_000;

#[allow(dead_code)]
/// Initialize the `Whisper` model.
pub fn init_model(model_path: &str) -> VirgilResult<WhisperState> {
    let span = span!(Level::TRACE, "init_model");
    let _enter = span.enter();

    let model_ctx =
        WhisperContext::new_with_params(model_path, WhisperContextParameters::default())?;
    let model = model_ctx.create_state().map_err(|e| e)?;
    info!("Model created: {model:?}");

    Ok(model)
}

#[allow(dead_code)]
/// Converts audio data to text using the provided `Whisper` model and parameters.
pub fn transcribe(
    model: &mut WhisperState,
    params: FullParams,
    audio_data: &[f32],
) -> VirgilResult<String> {
    model.full(params, audio_data)?;
    let mut transcript = String::with_capacity(2048);
    let num_segments = model.full_n_segments()?;
    for i in 0..num_segments {
        let segment = model.full_get_segment_text(i)?;
        transcript.push_str(&segment);
    }
    Ok(transcript)
}

#[allow(dead_code)]
/// Check for the specified wake words in the audio data.
pub fn detect_wake_words(
    model: &mut WhisperState,
    params: FullParams,
    audio_data: &[f32],
    wake_words: &Vec<String>,
) -> VirgilResult<bool> {
    let span = span!(Level::TRACE, "detect_wake_words");
    let _enter = span.enter();

    let transcript = transcribe(model, params, audio_data)?.to_lowercase();
    for word in wake_words {
        if transcript.contains(&word.to_lowercase()) {
            info!("Wake word detected: {word}");
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Error)]
#[error("MicrophoneConfigError: {0}")]
pub struct MicrophoneConfigError(String);

#[allow(dead_code)]
/// Initializes the microphone.
pub fn init_microphone(audio_data_tx: mpsc::Sender<Vec<f32>>) -> VirgilResult<Stream> {
    let span = span!(Level::TRACE, "init_microphone");
    let _enter = span.enter();

    // Initialize microphone
    let host = cpal::default_host();
    let microphone = host
        .default_input_device()
        .ok_or_else(|| MicrophoneConfigError("Default input device not found".into()))?;
    let config = microphone
        .supported_input_configs()?
        .next()
        .ok_or_else(|| MicrophoneConfigError("No supported configs found".into()))?
        .with_sample_rate(SampleRate(EXPECTED_SAMPLE_RATE as u32))
        .config();
    info!("Microphone initalized");

    // Initialize input stream
    fn input_stream_listener(sender: mpsc::Sender<Vec<f32>>, data: &[f32]) {
        let span = span!(Level::TRACE, "input_stream_listener");
        let _enter = span.enter();
        sender
            .try_send(data.into())
            .map_err(|e| error!("Unable to send audio data: {e}"))
            .unwrap()
    }
    let stream = microphone
        .build_input_stream(
            &config,
            move |data: &[f32], _: &InputCallbackInfo| {
                input_stream_listener(audio_data_tx.clone(), data)
            },
            move |err| error!("{err}"),
            None,
        )
        .map_err(|e| error!("Unable to initialize microphone input stream: {e}"))
        .unwrap();

    Ok(stream)
}

/// Accumulates audio data until there are the enough samples for the specified duration.
pub async fn accumulate_audio_data(
    sender: mpsc::Sender<Vec<f32>>,
    mut receiver: mpsc::Receiver<Vec<f32>>,
    duration_ms: usize,
) -> VirgilResult<()> {
    let desired_len = (duration_ms / 1000) * EXPECTED_SAMPLE_RATE;
    let mut accumulated_data = Vec::with_capacity(desired_len);
    while let Some(data) = receiver.recv().await {
        let accumulated = accumulated_data.len();
        let to_add = data.len();
        let new_len = accumulated + to_add;

        // Accumulate audio data until desired length is reached
        if new_len < desired_len {
            accumulated_data.extend_from_slice(&data);
            continue;
        }
        // More samples than desired
        else if new_len > desired_len {
            // Send desired number of samples
            accumulated_data.extend_from_slice(&data[0..desired_len]);
            sender.send(accumulated_data.clone()).await?;

            // Reset accumulated_data and fill with extra samples
            accumulated_data.clear();
            accumulated_data.extend_from_slice(&data[desired_len..]);
        }
        // Exact number of samples (as `desired_len`)
        else {
            accumulated_data.extend_from_slice(&data);
            sender.send(accumulated_data.clone()).await?;
            accumulated_data.clear();
        }
    }
    Ok(())
}
