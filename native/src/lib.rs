mod messages;
mod state;

use std::{
    ffi,
    ptr::slice_from_raw_parts,
    sync::{LazyLock, Mutex},
};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

// FIXME: Forward errors to Flutter!

#[derive(Default)]
struct RustError {
    errors: Vec<String>,
}

static ERRORS: LazyLock<Mutex<RustError>> = LazyLock::new(|| Mutex::new(RustError::default()));

fn add_error(error: String) {
    let mut errors = ERRORS.lock().unwrap();
    errors.errors.push(error);
}

/// Represents the library's state.
#[derive(Default)]
struct Model {
    state: Option<WhisperState>,
    audio_data: Option<Vec<f32>>,
    transcript: Option<String>,
}

/// The state maintained by the library.
static MODEL: LazyLock<Mutex<Model>> = LazyLock::new(|| Mutex::new(Model::default()));

/// Loads the Whisper model from the given path.
#[unsafe(no_mangle)]
pub fn load_model_(path: *const u8, len: u64) {
    if path.is_null() || len == 0 {
        add_error(format!("Invalid path: ({:?}, len = {})", path, len));
        return;
    }
    let len = len as usize;
    let slice = unsafe {
        slice_from_raw_parts(path, len)
            .as_ref()
            .ok_or_else(|| {
                add_error("Unable to build slice from path pointer".into());
                return;
            })
            .unwrap()
    };
    let model_path = String::from_utf8(slice.into())
        .map_err(|e| add_error(format!("Failed to build path into String: {e}")))
        .unwrap();

    // Load model
    let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
        .map_err(|e| add_error(format!("Unable to load model: {e}")))
        .unwrap();
    let state = ctx
        .create_state()
        .map_err(|e| add_error(format!("Unable to create state: {e}")))
        .unwrap();
    let mut model = MODEL.lock().unwrap();
    model.state = Some(state);
}

/// Updates the audio data to be transcribed.
#[unsafe(no_mangle)]
pub fn update_audio_data_(audio_data: *const ffi::c_float, len: u64) {
    if audio_data.is_null() || len == 0 {
        add_error(format!(
            "Invalid data: (audio_data = {audio_data:?}, len = {len})",
        ));
        return;
    }
    let len = len as usize;
    let audio_data = unsafe {
        slice_from_raw_parts(audio_data, len)
            .as_ref()
            .ok_or_else(|| {
                add_error(format!(
                    "Unable to create slice from data: (audio_data = {audio_data:?}, len = {len})"
                ));
                return;
            })
            .unwrap()
    };

    // Update data
    let mut model = MODEL.lock().unwrap();
    if let Some(data) = &mut model.audio_data {
        data.clear();
        data.extend_from_slice(audio_data);
    } else {
        model.audio_data = Some(audio_data.into());
    }
}

/// Checks if any wake words are present in the provided audio data.
#[unsafe(no_mangle)]
pub fn detect_wake_words_() -> bool {
    let mut model = MODEL.lock().unwrap();
    if let Some(audio_data) = &model.audio_data {
        // Run the model
        let mut transcript = String::with_capacity(256);
        run_model_(audio_data, &mut transcript);

        // Check transcript for wake words
        let lowered = transcript.to_lowercase();
        for wake_word in WAKE_WORDS {
            let idx = lowered.find(&wake_word.to_lowercase());
            if let Some(_idx) = idx {
                // Remove the wake word from the transcript
                // transcript.drain(idx..wake_word.len());

                // Update the model's transcript
                if let Some(model_transcript) = &mut model.transcript {
                    model_transcript.clear();
                    model_transcript.push_str(&transcript);
                } else {
                    model.transcript = Some(transcript.into())
                }

                return true;
            }
        }
    }

    false
}

/// Transcribes the audio data.
#[unsafe(no_mangle)]
pub fn transcribe_(out_len: *mut u64) -> *const u8 {
    if out_len.is_null() {
        return std::ptr::null();
    }
    let mut transcript = String::with_capacity(1024);
    unsafe {
        *out_len = transcript.len() as u64;
    }
    let ptr = transcript.as_mut_ptr();
    std::mem::forget(transcript);
    ptr
}

/// Frees the memory used by Rust's transcript.
#[unsafe(no_mangle)]
pub fn free_transcript_(ptr: *mut u8, len: u64) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    unsafe {
        let _ = String::from_raw_parts(ptr, len, len);
    }
}

// TODO: Make this a part of `Model` and allow Flutter to change it.
//
/// The wake words recognized by the app.
const WAKE_WORDS: [&str; 1] = ["Wake"];

/// Runs the model
fn run_model_(audio_data: &[f32], transcript: &mut String) {
    let mut model = MODEL.lock().unwrap();
    if let Some(state) = &mut model.state {
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        state.full(params, audio_data).unwrap();

        // Get the results
        let num_segments = state.full_n_segments().unwrap();
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i).unwrap();
            transcript.push_str(&segment);
        }
    }
}
