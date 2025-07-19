use std::{
    any::Any,
    ffi,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::state::{
    WakeWordDetection, detect_wake_words, load_model, transcribe, update_audio_data,
};

// ==================================================================
//                              Messages
// ==================================================================

/// Represents messages sent **from** Dart **to** Rust.
pub trait Message {
    fn as_any(self) -> Box<dyn Any>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

/// The various message types.
#[derive(Debug, Encode, Decode)]
pub enum MessageType {
    /// Load the model from the given path.
    LoadModel,

    /// Update the audio data to be transcribed.
    UpdateAudioData,

    // FIXME: Create separate message for setting wake words
    //
    /// Detect for the given wake words in the audio data.
    DetectWakeWords,

    /// Transcribes the audio data.
    Transcribe,
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0 => MessageType::LoadModel,
            1 => MessageType::UpdateAudioData,
            2 => MessageType::DetectWakeWords,
            3 => MessageType::Transcribe,
            _ => unreachable!(),
        }
    }
}
#[derive(Debug, Encode, Decode)]
pub struct LoadModel(String);
impl Message for LoadModel {}

#[derive(Debug, Encode, Decode)]
pub struct UpdateAudioData(Vec<f32>);
impl Message for UpdateAudioData {}

#[derive(Debug, Encode, Decode)]
pub struct DetectWakeWords(Vec<String>);
impl Message for DetectWakeWords {}

#[derive(Debug, Encode, Decode)]
pub struct Transcribe;
impl Message for Transcribe {}

// ==================================================================
//                              Responses
// ==================================================================

#[derive(Debug, Encode, Decode)]
pub struct TextResponse(String);

#[derive(Debug, Encode, Decode)]
pub struct WakeWordResponse(WakeWordDetection);

#[derive(Debug, Encode, Decode)]
pub struct ErrorResponse(String);

/// The various response types.
#[derive(Debug, Encode, Decode)]
pub enum ResponseType {
    Text,
    WakeWord,
    Error,
}

// ==================================================================
//                              Public API
// ==================================================================

/// Sends the specified message *to* Rust *from* Dart.
///
/// This function will return return a `Response` pointer that must be deallocated using the
/// [free_response] function.
#[unsafe(no_mangle)]
pub fn send_message_to_rust(
    msg_type: u8,
    msg_ptr: *const ffi::c_void,
    msg_len: usize,
    resp_len: *mut usize,
) -> *mut ffi::c_void {
    // Decode message
    let kind: MessageType = msg_type.into();
    let message = match kind {
        MessageType::LoadModel => {
            let message: LoadModel = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::UpdateAudioData => {
            let message: UpdateAudioData = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::DetectWakeWords => {
            let message: DetectWakeWords = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::Transcribe => Transcribe.as_any(),
    };

    // Respond to message
    match kind {
        MessageType::LoadModel => {
            // Loads the model
            let message = message.concrete::<LoadModel>();
            let model_path = &message.0;
            load_model(model_path)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();

            // Return serialized response
            let response = TextResponse(format!("Model path set to: {model_path}"));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
        }
        MessageType::UpdateAudioData => {
            // Updates the audio data
            let message = message.concrete::<UpdateAudioData>();
            let new_data = &message.0;
            update_audio_data(new_data)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();

            // Return serialized response
            let response = TextResponse(format!(
                "Audio data updated with {} samples",
                new_data.len()
            ));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
        }
        MessageType::DetectWakeWords => {
            // Detects the wake word
            let message = message.concrete::<DetectWakeWords>();
            let wake_words = &message.0;
            let detection = detect_wake_words(wake_words)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();

            // Return serialized response
            let response = WakeWordResponse(detection);
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
        }
        MessageType::Transcribe => {
            // Transcribes the audio data
            let transcript = transcribe()
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();

            // Return serialized response
            let response = TextResponse(transcript);
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_len))
                .unwrap();
        }
    };
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

// ==================================================================
//                              Private API
// ==================================================================

/// Serialize the given value.
///
/// # Note
/// The callier must free the the returned pointer with [free_response].
fn serialize<T: Encode>(value: T, len_ptr: *mut usize) -> Result<*mut ffi::c_void, String> {
    let mut bytes = Vec::<u8>::with_capacity(size_of_val(&value));
    let written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )
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
        decode_from_slice(slice, bincode::config::standard().with_fixed_int_encoding())
            .map_err(|e| e.to_string())?;
    Ok(decoded)
}

/// Returns an error `Response`.
fn rust_error(details: String, resp_len: *mut usize) -> *mut ffi::c_void {
    let error = ErrorResponse(details);
    serialize(error, resp_len).unwrap()
}

/// Allows for convenient downcasting.
trait Concrete {
    fn concrete<'a, T>(&'a self) -> &'a T
    where
        T: Sized + 'static;
}

impl Concrete for Box<dyn Any> {
    fn concrete<'a, T>(&'a self) -> &'a T
    where
        T: Sized + 'static,
    {
        self.downcast_ref().unwrap()
    }
}
