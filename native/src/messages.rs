use std::{
    ffi,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::state::{
    WakeWordDetection, detect_wake_words, load_model, transcribe, update_audio_data,
};

/// Represents messages sent **from** Dart **to** Rust.
#[derive(Debug, Encode, Decode)]
pub enum Message {
    /// Load the model from the given path.
    LoadModel(String),

    /// Update the audio data to be transcribed.
    UpdateAudioData(Vec<f32>),

    // FIXME: Create separate message for setting wake words
    //
    /// Detect for the given wake words in the audio data.
    DetectWakeWords(Vec<String>),

    /// Transcribes the audio data sent through `Message::UpdateAudioData`.
    Transcribe,
}

/// Represents the response **from** Rust **to** a Dart message.
#[derive(Debug, Encode, Decode)]
pub struct Response {
    /// The status of the response.
    status: ResponseStatus,

    /// Any additional details associated with the response.
    details: ResponseDetails,
}

/// The status of the response from Rust.
#[derive(Debug, Encode, Decode)]
pub enum ResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Encode, Decode)]
pub enum ResponseDetails {
    Text(String),
    WakeWordDetected(WakeWordDetection),
}

/// Sends the specified message *to* Rust *from* Dart.
///
/// This function will return return a `Response` pointer that must be deallocated using the
/// [free_response] function.
#[unsafe(no_mangle)]
pub fn send_message_to_rust(
    msg_ptr: *const ffi::c_void,
    msg_len: usize,
    resp_len: *mut usize,
) -> *mut ffi::c_void {
    // Decode message
    let message: Message = deserialize(msg_ptr, msg_len)
        .map_err(|e| return rust_error(e, resp_len))
        .unwrap();

    // Respond to message
    let response = match message {
        Message::LoadModel(model_path) => {
            // Loads the model
            load_model(&model_path)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            Response {
                status: ResponseStatus::Success,
                details: ResponseDetails::Text(format!("Model path set to: {model_path}")),
            }
        }
        Message::UpdateAudioData(new_data) => {
            // Updates the audio data
            update_audio_data(&new_data)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            Response {
                status: ResponseStatus::Success,
                details: ResponseDetails::Text(format!(
                    "Audio data updated with {} samples",
                    new_data.len()
                )),
            }
        }
        Message::DetectWakeWords(wake_words) => {
            // Detects the wake word
            let detection = detect_wake_words(&wake_words)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            Response {
                status: ResponseStatus::Success,
                details: ResponseDetails::WakeWordDetected(detection),
            }
        }
        Message::Transcribe => {
            // Transcribes the audio data
            let transcript = transcribe()
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            Response {
                status: ResponseStatus::Success,
                details: ResponseDetails::Text(transcript),
            }
        }
    };

    // Return serialized response
    serialize(response, resp_len)
        .map_err(|e| return rust_error(e, resp_len))
        .unwrap()
}

/// Frees the memory allocated for a response sent by [send_message_to_rust].
#[unsafe(no_mangle)]
pub fn free_response(ptr: *const ffi::c_void, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let ptr: *mut u8 = ptr.cast_mut().cast();
        let _ = Box::from_raw(slice_from_raw_parts_mut(ptr, len));
    }
}

/// Serialize the given value.
///
/// # Note
/// The callier must free the the returned pointer with [free_response].
fn serialize<T: Encode>(value: T, len_ptr: *mut usize) -> Result<*mut ffi::c_void, String> {
    let mut bytes = Vec::<u8>::with_capacity(size_of_val(&value));
    let written = encode_into_slice(value, bytes.as_mut_slice(), bincode::config::standard())
        .map_err(|e| e.to_string())?;
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    unsafe { *len_ptr = written };
    Ok(response_ptr)
}

/// Deserialize the value represented by the given pointer and length.
fn deserialize<T: Decode<()>>(ptr: *const ffi::c_void, len: usize) -> Result<T, String> {
    let slice = unsafe {
        let ptr: *const u8 = ptr.cast();
        let safe_slice = slice_from_raw_parts(ptr, len).as_ref();
        if let Some(slice) = safe_slice {
            slice
        } else {
            return Err("Unable to convert `ptr` to a slice of bytes".into());
        }
    };

    let (decoded, _): (T, usize) =
        decode_from_slice(slice, bincode::config::standard()).map_err(|e| e.to_string())?;
    Ok(decoded)
}

/// Returns an error `Response`.
fn rust_error(details: String, resp_len: *mut usize) -> *mut ffi::c_void {
    let error = Response {
        status: ResponseStatus::Error,
        details: ResponseDetails::Text(details),
    };
    serialize(error, resp_len).unwrap()
}
