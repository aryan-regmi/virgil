use std::{ffi, ptr::slice_from_raw_parts};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};
use cpal::{
    InputCallbackInfo, SampleRate, Stream,
    traits::{DeviceTrait, HostTrait},
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{Level, error, info, span};
use whisper_rs::{FullParams, WhisperContext, WhisperContextParameters, WhisperState};

use crate::messages::Message;

pub type VirgilResult<T> = Result<T, anyhow::Error>;

/// The context passed around for FFI functions.
#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
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

/// The expected sample rate of the microphone.
pub const EXPECTED_SAMPLE_RATE: usize = 16_000;

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
        .try_with_sample_rate(SampleRate(EXPECTED_SAMPLE_RATE as u32))
        .ok_or_else(|| MicrophoneConfigError(format!("No supported configs found with the the specified sample rate: {EXPECTED_SAMPLE_RATE} Hz")))?
        .config();

    // Initialize input stream
    fn input_stream_listener(sender: mpsc::Sender<Vec<f32>>, data: &[f32]) {
        let span = span!(Level::TRACE, "input_stream_listener");
        let _enter = span.enter();
        sender
            .try_send(data.into())
            .map_err(|e| error!("Unable to send audio data: {e}"))
            .unwrap()
    }
    let stream = microphone.build_input_stream(
        &config,
        move |data: &[f32], _: &InputCallbackInfo| {
            input_stream_listener(audio_data_tx.clone(), data)
        },
        move |err| error!("MicrophoneListenerError: {err}"),
        None,
    )?;

    info!("Microphone initalized");
    Ok(stream)
}

pub struct SendStream(pub Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}
