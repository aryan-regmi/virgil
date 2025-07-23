//! Defines and manages the state of the library.

use std::sync::{LazyLock, Mutex};

use bincode::{Decode, Encode};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

/// The state of the Whisper model.
pub static MODEL_STATE: LazyLock<Mutex<Option<WhisperState>>> = LazyLock::new(|| Mutex::new(None));

/// The wake words to listen for.
pub static WAKE_WORDS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(vec![]));

/// Loads the Whisper model from the given path.
pub fn load_model(model_path: &str) -> Result<(), String> {
    // Create the model
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| e.to_string())?;
    let state = ctx.create_state().map_err(|e| e.to_string())?;

    // Store the model
    let mut model = MODEL_STATE.lock().map_err(|e| e.to_string())?;
    *model = Some(state);
    Ok(())
}

/// Sets the wake words to listen for.
pub fn set_wake_words(words: &Vec<String>) -> Result<(), String> {
    let mut wake_words = WAKE_WORDS.lock().map_err(|e| e.to_string())?;
    wake_words.clear();
    wake_words.extend_from_slice(&words);
    Ok(())
}

/// Result of wake word detection.
#[derive(Debug, Encode, Decode)]
pub struct WakeWordDetection {
    /// Whether or not a wake word was detected.
    pub detected: bool,

    /// The start index in the transcript of the detected word.
    pub start_idx: usize,

    /// The end index in the transcript of the detected word.
    pub end_idx: usize,
}

/// Checks if any wake words are present in the provided audio data.
///
/// # Note
/// This should only be called after [update_audio_data].
pub fn detect_wake_words(audio_data: &[f32]) -> Result<WakeWordDetection, String> {
    let wake_words = WAKE_WORDS.lock().map_err(|e| e.to_string())?;
    if !audio_data.is_empty() {
        // Get transcript by running model
        let transcript = run_model(&audio_data)?.to_lowercase();

        // Check for wake words
        for phrase in &*wake_words {
            if let Some(idx) = transcript.find(&phrase.to_lowercase()) {
                return Ok(WakeWordDetection {
                    detected: true,
                    start_idx: idx,
                    end_idx: phrase.len(),
                });
            }
        }
    }

    Ok(WakeWordDetection {
        detected: false,
        start_idx: 0,
        end_idx: 0,
    })
}

/// Transcribes the audio data.
pub fn transcribe(audio_data: &[f32]) -> Result<String, String> {
    if !audio_data.is_empty() {
        run_model(&audio_data)
    } else {
        Ok("".into())
    }
}

/// Runs the stored model with the given audio data.
fn run_model(audio_data: &[f32]) -> Result<String, String> {
    let mut model = MODEL_STATE.lock().map_err(|e| e.to_string())?;

    if let Some(state) = &mut *model {
        // Run model
        state
            .full(
                FullParams::new(SamplingStrategy::Greedy { best_of: 1 }),
                audio_data,
            )
            .map_err(|e| e.to_string())?;

        // Save results
        let mut transcript = String::with_capacity(1026);
        let num_segments = state.full_n_segments().unwrap();
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i).unwrap();
            transcript.push_str(&segment);
        }
        Ok(transcript.trim().into())
    } else {
        Err("Invalid model state".into())
    }
}
