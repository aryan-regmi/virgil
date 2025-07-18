use std::{
    ffi,
    fs::File,
    ptr::slice_from_raw_parts_mut,
    sync::{Mutex, OnceLock},
};

use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;
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

/// The size of the audio buffer.
// const AUDIO_BUFFER_SIZE: usize = 44_100;
const AUDIO_BUFFER_SIZE: usize = 1024 * 8;

/// The audio data array used for all library calls.
fn audio_array() -> &'static Mutex<Vec<f32>> {
    static ARRAY: OnceLock<Mutex<Vec<f32>>> = OnceLock::new();
    ARRAY.get_or_init(|| Mutex::new(vec![0_f32; AUDIO_BUFFER_SIZE]))
}

/// Loads the model from the given path.
#[unsafe(no_mangle)]
pub fn load_model(path: *const ffi::c_char) {
    let path_to_model = unsafe {
        ffi::CStr::from_ptr(path)
            .to_str()
            .map_err(|err| error!("Invalid path: {err}"))
            .unwrap()
    };

    // Load the context and model
    let ctx = WhisperContext::new_with_params(&path_to_model, WhisperContextParameters::default())
        .map_err(|err| error!("Failed to load model: {err}"))
        .unwrap();
    info!("Model loaded");

    // Create state that can later be run
    let state = ctx
        .create_state()
        .map_err(|err| error!("Failed to create valid model state: {err}"))
        .unwrap();

    // Update the model
    {
        model().lock().unwrap().state = Some(state);
    }
}

/// Checks if the given wake word is detected in the audio data.
#[unsafe(no_mangle)]
pub fn wake_word_detected(wake_words: *const ffi::c_char) -> bool {
    let text = {
        let text: *const ffi::c_char = transcribe();
        unsafe { ffi::CStr::from_ptr(text) }
            .to_str()
            .map_err(|err| error!("Invalid transcript: {err}"))
            .unwrap()
    };

    let words: String;
    let wake_words = {
        words = unsafe {
            ffi::CStr::from_ptr(wake_words)
                .to_str()
                .map_err(|err| error!("Invalid wake words string: {err}"))
                .unwrap()
                .into()
        };
        words.split("\n")
    };

    for wake_word in wake_words {
        if text.contains(&wake_word.to_lowercase()) {
            let mut out = File::create("out.txt").unwrap();
            info!("Wake word detected: {}", wake_word);
            return true;
        }
    }

    false
}

/// Transcribes the given raw audio data into text.
#[unsafe(no_mangle)]
pub fn transcribe() -> *const ffi::c_char {
    let audio_data = audio_array().lock();
    let audio_data = audio_data.as_ref().unwrap();
    let mut model = model().lock();
    let model = model
        .as_mut()
        .map_err(|err| error!("Invalid model: {err}"))
        .unwrap();

    if let Some(state) = &mut model.state {
        // Create a params object and run model
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        state
            .full(params, audio_data)
            .map_err(|err| error!("Failed to run model: {err}"))
            .unwrap();

        // Transcribe the text
        let mut transcription = String::with_capacity(256);
        let num_segments = state
            .full_n_segments()
            .map_err(|err| error!("Failed to get number of segments: {err}"))
            .unwrap();
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|err| error!("Failed to get segment: {err}"))
                .unwrap();
            transcription.push_str(&segment);
        }
        model.transcript = ffi::CString::new(transcription.trim())
            .map_err(|err| error!("Invalid transcript: {err}"))
            .unwrap();
    }

    model.transcript.as_ptr()
}

/// Gets the audio data array.
#[unsafe(no_mangle)]
pub fn get_audio_data_array() -> *const ffi::c_float {
    audio_array().lock().unwrap().as_ptr() as *const ffi::c_float
}

/// Sets the value at the specified index of the audio array.
#[unsafe(no_mangle)]
pub fn set_audio_data(idx: usize, value: f32) {
    if idx > AUDIO_BUFFER_SIZE {}
    audio_array().lock().unwrap()[idx] = value;
}

/// Resets/empties the audio data.
#[unsafe(no_mangle)]
pub fn reset_audio_data_array() {
    let array_ptr = get_audio_data_array().cast_mut();
    let slice = unsafe {
        slice_from_raw_parts_mut(array_ptr, AUDIO_BUFFER_SIZE)
            .as_mut()
            .unwrap()
    };
    slice.fill_with(|| 0.0);
}

#[unsafe(no_mangle)]
pub fn set_logger() {
    let subscriber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| {
            eprintln!("Unable to set global default subscriber");
            err
        })
        .expect("Unable to setup logger");
}

#[cfg(test)]
mod tests {
    use std::{error::Error, ffi::CString, ptr::slice_from_raw_parts_mut, str::FromStr};
    use tracing_subscriber::{self, FmtSubscriber};

    use super::*;

    fn setup() -> Result<(), Box<dyn Error>> {
        let subscriber = FmtSubscriber::new();

        tracing::subscriber::set_global_default(subscriber).map_err(|err| {
            eprintln!("Unable to set global default subscriber");
            err
        })?;

        Ok(())
    }

    const SAMPLE_AUDIO: [f32; 43] = [
        0.0023, 0.0056, 0.0098, 0.0154, 0.0221, 0.0276, 0.0302, 0.0284, 0.0229, 0.0141, 0.0032,
        -0.0095, -0.0223, -0.0336, -0.0421, -0.0467, -0.0462, -0.0407, -0.0306, -0.0167, -0.0008,
        0.0164, 0.0323, 0.0452, 0.0534, 0.0560, 0.0526, 0.0433, 0.0290, 0.0107, -0.0095, -0.0293,
        -0.0464, -0.0585, -0.0637, -0.0609, -0.0504, -0.0336, -0.0129, 0.0092, 0.0298, 0.0464,
        0.0566,
    ];

    fn fill_audio_data_with_samples() {
        let data_ptr = get_audio_data_array().cast_mut();
        let audio_data = unsafe { slice_from_raw_parts_mut(data_ptr, AUDIO_BUFFER_SIZE).as_mut() };
        if let Some(audio_data) = audio_data {
            let mut count = 0;
            // Fill the array by repeating the sample
            for i in 0..AUDIO_BUFFER_SIZE {
                if count == SAMPLE_AUDIO.len() {
                    count = 0;
                }
                audio_data[i] = SAMPLE_AUDIO[count];
                count += 1;
            }
        }
    }

    fn audio_samples_from_wav() {
        let mut reader = hound::WavReader::open("test_assets/M1F1-float32-AFsp.wav").unwrap();
        let samples: Vec<f32> = reader
            .samples::<f32>()
            .map(|e| e.unwrap_or_else(|_| 0.0))
            .collect();

        let data_ptr = get_audio_data_array().cast_mut();
        let audio_data = unsafe { slice_from_raw_parts_mut(data_ptr, AUDIO_BUFFER_SIZE).as_mut() };
        if let Some(audio_data) = audio_data {
            let mut count = 0;
            // Fill the array by repeating the sample
            for i in 0..AUDIO_BUFFER_SIZE {
                if count == samples.len() {
                    count = 0;
                }
                audio_data[i] = samples[count];
                count += 1;
            }
        }
    }

    #[test]
    fn can_load() {
        setup().unwrap();
        let model_path = CString::from_str("test_assets/ggml-tiny.bin").unwrap();
        load_model(model_path.as_ptr());
        assert!(model().lock().unwrap().state.is_some());
    }

    #[test]
    fn can_access_audio_array() {
        // setup().unwrap();
        fill_audio_data_with_samples();

        let data_ptr = get_audio_data_array().cast_mut();
        // let audio_data = unsafe { Vec::from_raw_parts(data_ptr, SAMPLE_RATE, SAMPLE_RATE) };
        let audio_data =
            unsafe { slice_from_raw_parts_mut(data_ptr, AUDIO_BUFFER_SIZE).as_ref() }.unwrap();
        assert!(audio_data[0..SAMPLE_AUDIO.len()] == SAMPLE_AUDIO);

        reset_audio_data_array();
    }

    #[test]
    fn can_reset_audio_array() {
        // setup().unwrap();

        {
            fill_audio_data_with_samples();

            let data_ptr = get_audio_data_array().cast_mut();
            let audio_data =
                unsafe { slice_from_raw_parts_mut(data_ptr, AUDIO_BUFFER_SIZE).as_ref() }.unwrap();
            assert!(audio_data[0..SAMPLE_AUDIO.len()] == SAMPLE_AUDIO);
        }

        reset_audio_data_array();

        {
            let data_ptr = get_audio_data_array().cast_mut();
            let audio_data =
                unsafe { slice_from_raw_parts_mut(data_ptr, AUDIO_BUFFER_SIZE).as_ref() }.unwrap();
            assert!(audio_data.iter().all(|v| *v == 0.0));
        }
    }

    #[test]
    fn can_transcribe() {
        // setup().unwrap();
        audio_samples_from_wav();

        let model_path = CString::from_str("test_assets/ggml-tiny.bin").unwrap();
        load_model(model_path.as_ptr());

        let text = {
            let text_ptr = transcribe();
            unsafe { ffi::CStr::from_ptr(text_ptr) }.to_str().unwrap()
        };
        assert_eq!(text, "Happy birthday to my offering corn.");

        reset_audio_data_array();
    }
}
