use std::{ffi, ptr::slice_from_raw_parts, thread, time::Duration};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};
use cpal::{
    InputCallbackInfo, SampleRate, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use thiserror::Error;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Receiver},
};
use tracing::{Level, debug, error, info, span};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::{messages::Message, port::send_text_to_dart};

pub type VirgilResult<T> = Result<T, anyhow::Error>;

/// The context passed around for FFI functions.
#[derive(Encode, Decode, Clone)]
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

/// A `Stream` that can be sent through threads safely.
pub struct SendStream(pub Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub fn listen_low_power_mode(ctx: Context, mut model: WhisperState, listen_duration_ms: u64) {
    let span = span!(Level::TRACE, "low_power_listening");
    let _enter = span.enter();

    let listen_duration = Duration::from_millis(listen_duration_ms);

    // Init tokio runtime
    let rt = Runtime::new().unwrap();

    // Setup channels for communication
    let (input_audio_tx, input_audio_rx) = mpsc::channel::<Vec<f32>>(EXPECTED_SAMPLE_RATE);
    let (process_duration_tx, process_duration_rx) = mpsc::channel::<Duration>(1);
    let (mic_duration_tx, mic_duration_rx) = mpsc::channel::<Duration>(1);

    // Initalize microphone
    let mic = SendStream(
        init_microphone(input_audio_tx.clone())
            .map_err(|e| error!("{e}"))
            .unwrap(),
    );

    // Listen to the microphone for the specified amount of time
    rt.spawn(listen_to_mic(mic, listen_duration, mic_duration_rx));

    let ctx2 = ctx.clone();
    thread::spawn(move || {
        rt.block_on(async move {
            tokio::spawn(process_audio(
                input_audio_rx,
                process_duration_rx,
                listen_duration,
                move |audio_data| {
                    let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

                    // Switch to active listening if wake word detected
                    let wake_word_detected =
                        detect_wake_words(&mut model, params.clone(), audio_data, &ctx2.wake_words)
                            .map_err(|e| error!("{e}"))
                            .unwrap();
                    if wake_word_detected {
                        // Listen for longer (3 seconds)
                        const ACTIVE_LISTEN_DURATION: Duration = Duration::from_secs(3);
                        mic_duration_tx
                            .blocking_send(ACTIVE_LISTEN_DURATION)
                            .map_err(|e| error!("{e}"))
                            .unwrap();
                        process_duration_tx
                            .blocking_send(ACTIVE_LISTEN_DURATION)
                            .map_err(|e| error!("{e}"))
                            .unwrap();

                        // Transcribe data
                        let text = transcribe(&mut model, params, audio_data)
                            .map_err(|e| error!("Unable to process audio: {e}"))
                            .unwrap();

                        // Send transcript to Dart
                        send_text_to_dart(text)
                            .map_err(|e| error!("Unable to send text to Dart: {e}"))
                            .unwrap();
                        debug!("Transcript sent");

                        // Reset the listen/process durations
                        mic_duration_tx
                            .blocking_send(Duration::from_millis(listen_duration_ms))
                            .map_err(|e| error!("{e}"))
                            .unwrap();
                        process_duration_tx
                            .blocking_send(Duration::from_millis(listen_duration_ms))
                            .map_err(|e| error!("{e}"))
                            .unwrap();
                    }
                },
            ));

            // Keep tokio runtime alive
            futures::future::pending::<()>().await
        });
    });
}

async fn process_audio<F>(
    mut input_audio_rx: mpsc::Receiver<Vec<f32>>,
    mut process_duration_rx: mpsc::Receiver<Duration>,
    default_process_duration: Duration,
    mut on_accumulated: F,
) where
    F: FnMut(&Vec<f32>),
{
    let span = span!(Level::TRACE, "process_audio");
    let _enter = span.enter();

    let desired_num_samples =
        (default_process_duration.as_millis() as usize / 1000) * EXPECTED_SAMPLE_RATE + 200;
    let mut accumulated_audio = Vec::with_capacity(desired_num_samples);

    let mut process_duration = default_process_duration;
    loop {
        // FIXME: Handle multiple channels
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

                // Run callback once desired number of samples are accumulated
                on_accumulated(&audio_data);

                // Reset accumulated data and fill with remaining/overflowing samples
                debug!("Accumulated data reset");
                accumulated_audio.clear();
                accumulated_audio.extend_from_slice(&audio_data[end_idx..]);

                continue;
            }
        }

        if let Ok(duration) = process_duration_rx.try_recv() {
            process_duration = duration;
        }

        std::thread::sleep(process_duration);
    }
}

async fn listen_to_mic(
    mic: SendStream,
    default_listen_duration: Duration,
    mut duration_update_rx: Receiver<Duration>,
) {
    let span = span!(Level::TRACE, "listen_to_mic");
    let _enter = span.enter();

    mic.0
        .play()
        .map_err(|e| error!("Failed to start listening to mic: {e}"))
        .unwrap();
    info!("Listening to microphone...");

    let mut listen_duration = default_listen_duration;
    loop {
        if let Ok(duration) = duration_update_rx.try_recv() {
            listen_duration = duration;
        }
        tokio::time::sleep(listen_duration).await
    }
}
