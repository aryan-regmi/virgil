use std::{
    ffi,
    ptr::slice_from_raw_parts,
    sync::{LazyLock, Mutex},
};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

// FIXME: Forward errors to Flutter!

/// Represents the library's state.
#[derive(Default)]
struct Model {
    state: Option<WhisperState>,
    audio_data: Option<Vec<f32>>,
}

static MODEL: LazyLock<Mutex<Model>> = LazyLock::new(|| Mutex::new(Model::default()));

/// Loads the Whisper model from the given path.
#[unsafe(no_mangle)]
pub fn load_model(path: *const u8, len: u64) {
    if path.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    let slice = unsafe { slice_from_raw_parts(path, len).as_ref().unwrap() };
    let model_path = String::from_utf8(slice.into()).unwrap();

    // Load model
    let ctx =
        WhisperContext::new_with_params(&model_path, WhisperContextParameters::default()).unwrap();
    let state = ctx.create_state().unwrap();
    let mut model = MODEL.lock().unwrap();
    model.state = Some(state);
}

/// Updates the audio data to be transcribed.
#[unsafe(no_mangle)]
pub fn update_audio_data(audio_data: *const ffi::c_float, len: u64) {
    if audio_data.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    let audio_data = unsafe { slice_from_raw_parts(audio_data, len).as_ref().unwrap() };

    // Update data
    let mut model = MODEL.lock().unwrap();
    if let Some(data) = &mut model.audio_data {
        data.clear();
        data.extend_from_slice(audio_data);
    } else {
        model.audio_data = Some(audio_data.into());
    }
}

// TODO: Make this a part of `Model` and allow Flutter to change it.
//
/// The wake words recognized by the app.
const WAKE_WORDS: [&str; 1] = ["Wake"];

/// Checks if any wake words are present in the provided audio data.
#[unsafe(no_mangle)]
pub fn detect_wake_words() -> bool {
    let model = MODEL.lock().unwrap();
    if let Some(audio_data) = &model.audio_data {
        let mut transcript = String::with_capacity(256);

        // Run the model
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

        // Check transcript for wake words
        let lowered = transcript.to_lowercase();
        for wake_word in WAKE_WORDS {
            if lowered.contains(&wake_word.to_lowercase()) {
                return true;
            }
        }
    }

    false
}

// NOTE: Examples of how to send and receive data from byte buffers.
//
// /// Returns the model path to Dart.
// #[unsafe(no_mangle)]
// pub fn get_model_path(out_len: *mut u64) -> *mut u8 {
//     let path = MODEL_PATH.lock().unwrap();
//     if out_len.is_null() {
//         return std::ptr::null::<u8>().cast_mut();
//     }
//     unsafe { *out_len = path.len() as u64 }
//
//     // Allocate new buffer and copy the data into it
//     let mut buf = path.clone();
//     let ptr = buf.as_mut_ptr();
//     std::mem::forget(buf);
//
//     ptr
// }
//
// /// Frees the memory for returned in `get_model`.
// #[unsafe(no_mangle)]
// pub fn free_model_path(ptr: *mut u8, len: u64) {
//     if ptr.is_null() || len == 0 {
//         return;
//     }
//     let len = len as usize;
//     unsafe {
//         let _ = String::from_raw_parts(ptr, len, len);
//     }
// }
