use std::{
    ffi,
    ptr::slice_from_raw_parts,
    sync::{Mutex, OnceLock},
};

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

/// Represents the model used for speech recognition.
pub struct SpeechRecognitionModel {
    state: Option<WhisperState>,
    transcript: ffi::CString,
}

/// The model that is used for all library calls.
fn model() -> &'static Mutex<SpeechRecognitionModel> {
    static MODEL: OnceLock<Mutex<SpeechRecognitionModel>> = OnceLock::new();
    MODEL.get_or_init(|| {
        Mutex::new(SpeechRecognitionModel {
            state: None,
            transcript: ffi::CString::new(String::with_capacity(256)).unwrap(),
        })
    })
}

/// Loads the model from the given path.
#[unsafe(no_mangle)]
pub fn load_model(path: *const ffi::c_char) {
    let path_to_model = unsafe { ffi::CStr::from_ptr(path).to_str().expect("Invalid path") };

    // Load the context and model
    let ctx = WhisperContext::new_with_params(&path_to_model, WhisperContextParameters::default())
        .expect("failed to load model");

    // Create state that can later be run
    let state = ctx.create_state().expect("Failed to create state");

    // Update the model
    {
        model().lock().unwrap().state = Some(state);
    }
}

/// Checks if the given wake word is detected in the audio data.
#[unsafe(no_mangle)]
pub fn wake_word_detected(
    audio_data: *const ffi::c_float,
    num_samples: ffi::c_int,
    wake_words: *const *const ffi::c_char,
    num_wake_words: ffi::c_int,
) -> bool {
    let text = {
        let text: *const ffi::c_char = transcribe(audio_data, num_samples);
        unsafe { ffi::CStr::from_ptr(text) }
            .to_str()
            .expect("Invalid text")
    };

    let wake_words = {
        let words: &[*const ffi::c_char] = unsafe {
            slice_from_raw_parts(wake_words, num_wake_words as usize)
                .as_ref()
                .expect("Invalid wake words")
        };
        let mut wake_words: Vec<String> = Vec::with_capacity(num_wake_words as usize);
        for word in words {
            unsafe {
                wake_words.push(
                    ffi::CStr::from_ptr(*word)
                        .to_str()
                        .expect("Invalid wake word")
                        .into(),
                );
            }
        }
        wake_words
    };

    for wake_word in wake_words {
        if text.contains(&wake_word.to_lowercase()) {
            return true;
        }
    }

    false
}

/// Transcribes the given raw audio data into text.
#[unsafe(no_mangle)]
pub fn transcribe(audio_data: *const ffi::c_float, num_samples: ffi::c_int) -> *const ffi::c_char {
    let audio_data = unsafe { slice_from_raw_parts(audio_data, num_samples as usize).as_ref() };
    if let Some(audio_data) = audio_data {
        let mut model = model().lock();
        let model = model.as_mut().expect("Invalid model");

        if let Some(state) = &mut model.state {
            // Create a params object and run model
            let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            state.full(params, audio_data).expect("Failed to run model");

            // Transcribe the text
            let mut transcription = String::with_capacity(256);
            let num_segments = state
                .full_n_segments()
                .expect("failed to get number of segments");
            for i in 0..num_segments {
                let segment = state
                    .full_get_segment_text(i)
                    .expect("failed to get segment");
                transcription.push_str(&segment);
            }
            model.transcript = ffi::CString::new(transcription).expect("Invalid transcript");
            return model.transcript.as_ptr();
        }
    }

    model().lock().unwrap().transcript.as_ref().as_ptr()
}
