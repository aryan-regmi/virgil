//! Defines and manages the state of the library.

use std::sync::{LazyLock, Mutex};

use whisper_rs::{WhisperContext, WhisperContextParameters, WhisperState};

/// The expected sample rate for the audio data.
const EXPECTED_SAMPLE_RATE: usize = 44_100;

/// The state of the Whisper model.
static MODEL_STATE: LazyLock<Mutex<Option<WhisperState>>> = LazyLock::new(|| Mutex::new(None));

/// The current audio data to be processed.
static AUDIO_DATA: LazyLock<Mutex<Vec<f32>>> =
    LazyLock::new(|| Mutex::new(Vec::with_capacity(EXPECTED_SAMPLE_RATE)));

/// The current transcript.
static TRANSCRIPT: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(String::with_capacity(1024)));

/// Loads the Whisper model from the given path.
pub fn load_model(model_path: &str) -> Result<(), String> {
    // Create the model
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| e.to_string())?;
    let state = ctx.create_state().map_err(|e| e.to_string())?;

    // Store the model
    let mut model = MODEL_STATE.lock().map_err(|e| e.to_string())?;
    *model = Some(state);

    todo!()
}
