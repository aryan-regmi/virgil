use std::{
    ffi,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::state::load_model;

/// Represents messages sent **from** Dart **to** Rust.
#[derive(Debug, Encode, Decode)]
pub enum Message {
    /// Load the model from the given path.
    LoadModel(String),

    /// Update the audio data to be transcribed.
    UpdateAudioData(Vec<f32>),
}

/// Represents the response **from** Rust **to** a Dart message.
#[derive(Debug, Encode, Decode)]
pub struct Response {
    /// The status of the response.
    status: ResponseStatus,

    /// Any additional details associated with the response.
    details: String,
}

/// The status of the response from Rust.
#[derive(Debug, Encode, Decode)]
pub enum ResponseStatus {
    Success,
    Error,
}

// FIXME: Add `res_len: *mut usize` output param, so responses can be freed correctly!
// - Update `rust_error` to also change this length
//
/// Sends the specified message *to* Rust *from* Dart.
///
/// This function will return return a `Response` pointer that must be deallocated using the
/// [free_response] function.
#[unsafe(no_mangle)]
pub fn send_message_to_rust(msg_ptr: *const ffi::c_void, msg_len: usize) -> *mut ffi::c_void {
    // Decode message
    let message: Message = deserialize(msg_ptr, msg_len)
        .map_err(|e| return rust_error(e))
        .unwrap();

    // Respond to message
    let response = match message {
        Message::LoadModel(model_path) => {
            // Loads the model
            load_model(&model_path)
                .map_err(|e| return rust_error(e))
                .unwrap();
            Response {
                status: ResponseStatus::Success,
                details: format!("Model path set to: {model_path}"),
            }
        }
        Message::UpdateAudioData(items) => {
            // FIXME: update audio data
            Response {
                status: ResponseStatus::Success,
                details: format!("Audio data updated with {} samples", items.len()),
            }
        }
    };
    serialize(response)
        .map_err(|e| return rust_error(e))
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
fn serialize<T: Encode>(value: T) -> Result<*mut ffi::c_void, String> {
    let mut bytes = Vec::<u8>::with_capacity(size_of_val(&value));
    encode_into_slice(value, bytes.as_mut_slice(), bincode::config::standard())
        .map_err(|e| e.to_string())?;
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
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
fn rust_error(details: String) -> *mut ffi::c_void {
    let error = Response {
        status: ResponseStatus::Error,
        details,
    };
    return serialize(error).unwrap();
}
