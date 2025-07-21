use std::{
    any::Any,
    ffi,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::state::{
    WakeWordDetection, detect_wake_words, load_model, set_wake_words, transcribe, update_audio_data,
};

// ==================================================================
//                              Messages
// ==================================================================

/// Represents messages sent **from** Dart **to** Rust.
pub trait Message: Encode + Decode<()> {
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

    /// Set wake words.
    SetWakeWords,

    /// Update the audio data to be transcribed.
    UpdateAudioData,

    /// Detect for the given wake words in the audio data.
    DetectWakeWords,

    /// Transcribes the audio data.
    Transcribe,

    /// Message used for debugging.
    Debug,
}

// NOTE: Keep in sync with [MessageType]!
impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0 => MessageType::LoadModel,
            1 => MessageType::SetWakeWords,
            2 => MessageType::UpdateAudioData,
            3 => MessageType::DetectWakeWords,
            4 => MessageType::Transcribe,
            5 => MessageType::Debug,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Encode, Decode)]
pub struct LoadModel(String);
impl Message for LoadModel {}

#[derive(Debug, Encode, Decode)]
pub struct SetWakeWords(Vec<String>);
impl Message for SetWakeWords {}

#[derive(Debug, Encode, Decode)]
pub struct UpdateAudioData(Vec<f32>);
impl Message for UpdateAudioData {}

#[derive(Debug, Encode, Decode)]
pub struct DetectWakeWords;
impl Message for DetectWakeWords {}

#[derive(Debug, Encode, Decode)]
pub struct Transcribe;
impl Message for Transcribe {}

#[derive(Debug, Encode, Decode)]
pub struct DebugMessage(String);
impl Message for DebugMessage {}

// ==================================================================
//                              Responses
// ==================================================================

/// Represents responses sent **from** Rust **to** Dart.
pub trait Response: Encode + Decode<()> {
    fn byte_len(&self) -> usize;
}

/// The various response types.
#[derive(Debug, Encode, Decode)]
pub enum ResponseType {
    Text,
    WakeWord,
    Error,
}

impl Into<u8> for ResponseType {
    fn into(self) -> u8 {
        match self {
            ResponseType::Text => 0,
            ResponseType::WakeWord => 1,
            ResponseType::Error => 2,
        }
    }
}

/// Represents text responses sent **from** Rust **to** Dart.
#[derive(Debug, Encode, Decode)]
pub struct TextResponse(String);
impl Response for TextResponse {
    fn byte_len(&self) -> usize {
        self.0.len() + size_of::<Self>()
    }
}

/// Represents response to `DetectWakeWord` message sent **from** Rust **to** Dart.
#[derive(Debug, Encode, Decode)]
pub struct WakeWordResponse(WakeWordDetection);
impl Response for WakeWordResponse {
    fn byte_len(&self) -> usize {
        size_of::<WakeWordDetection>() + size_of::<Self>()
    }
}

/// Represents error responses sent **from** Rust **to** Dart.
#[derive(Debug, Encode, Decode)]
pub struct ErrorResponse(String);
impl Response for ErrorResponse {
    fn byte_len(&self) -> usize {
        self.0.len() + size_of::<Self>()
    }
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
    resp_type: *mut u8,
    resp_len: *mut usize,
) -> *mut ffi::c_void {
    // Validate inputs
    if msg_ptr.is_null() || resp_type.is_null() || resp_len.is_null() {
        return std::ptr::null_mut();
    }

    // Decode message
    let kind: MessageType = msg_type.into();
    let message = match kind {
        MessageType::LoadModel => {
            let message: LoadModel = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::SetWakeWords => {
            let message: SetWakeWords = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::UpdateAudioData => {
            let message: UpdateAudioData = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
            message.as_any()
        }
        MessageType::DetectWakeWords => DetectWakeWords.as_any(),
        MessageType::Transcribe => Transcribe.as_any(),
        MessageType::Debug => {
            let message: DebugMessage = deserialize(msg_ptr, msg_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
            message.as_any()
        }
    };

    // Respond to message
    match kind {
        MessageType::LoadModel => {
            // Loads the model
            let message = message.concrete::<LoadModel>();
            let model_path = &message.0;
            load_model(model_path)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();

            // Return serialized response
            unsafe { *resp_type = ResponseType::Text.into() };
            let response = TextResponse(format!("Model path set to: {model_path}"));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
        }
        MessageType::SetWakeWords => {
            // Set the wake words
            let message = message.concrete::<SetWakeWords>();
            set_wake_words(&message.0)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();

            // Return serialized response
            unsafe { *resp_type = ResponseType::WakeWord.into() };
            let response = TextResponse(format!("Wake words set to: [{}]", message.0.join(", ")));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
        }
        MessageType::UpdateAudioData => {
            // Updates the audio data
            let message = message.concrete::<UpdateAudioData>();
            let new_data = &message.0;
            update_audio_data(new_data)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();

            // Return serialized response
            unsafe { *resp_type = ResponseType::Text.into() };
            let response = TextResponse(format!(
                "Audio data updated with {} samples",
                new_data.len()
            ));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
        }
        MessageType::DetectWakeWords => {
            // Detects the wake word
            let detection = detect_wake_words()
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();

            // Return serialized response
            unsafe { *resp_type = ResponseType::WakeWord.into() };
            let response = WakeWordResponse(detection);
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
        }
        MessageType::Transcribe => {
            // Transcribes the audio data
            let transcript = transcribe()
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();

            // Return serialized response
            unsafe { *resp_type = ResponseType::Text.into() };
            let response = TextResponse(transcript);
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
                .unwrap();
        }
        MessageType::Debug => {
            // Return serialized response
            let message = message.concrete::<DebugMessage>();
            unsafe { *resp_type = ResponseType::Text.into() };
            let response = TextResponse(format!("Debug: '{}'", message.0));
            return serialize(response, resp_len)
                .map_err(|e| return rust_error(e, resp_type, resp_len))
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
fn serialize<T: Response>(value: T, len_ptr: *mut usize) -> Result<*mut ffi::c_void, String> {
    let mut bytes = vec![0; value.byte_len()];
    let written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )
    .map_err(|e| e.to_string())?;
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    if !len_ptr.is_null() {
        unsafe { *len_ptr = written };
    }
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
fn rust_error(details: String, resp_type: *mut u8, resp_len: *mut usize) -> *mut ffi::c_void {
    eprintln!("{}", details);
    if resp_type.is_null() || resp_len.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { *resp_type = ResponseType::Error.into() };
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
