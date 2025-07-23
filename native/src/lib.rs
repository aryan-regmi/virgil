use std::{
    ffi,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::{
    messages::{MessageStatus, RustMessage},
    state::Context,
};

mod messages;
mod state;

/// Frees the memory allocated by Rust.
#[unsafe(no_mangle)]
pub fn free_rust_ptr(ptr: *const ffi::c_void, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let ptr: *mut u8 = ptr.cast_mut().cast();
        let _ = Box::from_raw(slice_from_raw_parts_mut(ptr, len));
    }
}

/// Initalizes the application context.
#[unsafe(no_mangle)]
pub fn init_context(
    model_path: *const ffi::c_void,
    model_path_len: usize,
    wake_words: *const ffi::c_void,
    wake_words_len: usize,
) -> *mut ffi::c_void {
    // Decode model path and wake words
    let model_path: String = deserialize(model_path, model_path_len)
        .map_err(|e| return rust_error(e.to_string()))
        .unwrap();
    let wake_words: Vec<String> = deserialize(wake_words, wake_words_len)
        .map_err(|e| return rust_error(e.to_string()))
        .unwrap();

    // Encode context
    let transcript_capacity = 1024;
    let ctx = Context {
        model_path,
        wake_words,
        transcript: String::with_capacity(transcript_capacity),
    };
    let extra_byte_len = wake_words_len + model_path_len + transcript_capacity;
    serialize_message(RustMessage {
        status: MessageStatus::Success,
        byte_len: msg_size::<Context>(extra_byte_len),
        message: serialize_unchecked(ctx, extra_byte_len),
    })
    .map_err(|e| return rust_error(e.to_string()))
    .unwrap()
}

/// Starts listening to the microphone input and transcribes it.
#[unsafe(no_mangle)]
pub fn start_listening(ctx: *mut ffi::c_void, ctx_len: usize) -> *mut ffi::c_void {
    // Decode context
    let mut ctx: Context = deserialize(ctx, ctx_len)
        .map_err(|e| return rust_error(e.to_string()))
        .unwrap();
    let wake_words_len = ctx.wake_words.len();
    let model_path_len = ctx.model_path.len();

    // Run Virgil
    // let mut virgil = Virgil::new(&ctx.model_path, ctx.wake_words.clone())
    //     .map_err(|e| return rust_error(e.to_string()))
    //     .unwrap();
    // virgil
    //     .listen()
    //     .map_err(|e| return rust_error(e.to_string()))
    //     .unwrap();
    //
    // // Update transcript
    // ctx.transcript = virgil.transcript.clone();
    //
    // // Encode context
    // let extra_byte_len = wake_words_len + model_path_len + ctx.transcript.len();
    // serialize_message(RustMessage {
    //     status: MessageStatus::Success,
    //     byte_len: msg_size::<Context>(extra_byte_len),
    //     message: serialize_unchecked(ctx, extra_byte_len),
    // })
    // .map_err(|e| return rust_error(e.to_string()))
    // .unwrap()

    todo!()
}

/// Serialize the given message.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
fn serialize_message(value: RustMessage) -> Result<*mut ffi::c_void, String> {
    let byte_len = value.byte_len;
    let bytes = serialize(value, byte_len)?;
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    Ok(response_ptr)
}

/// Serialize the given encodable value.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
fn serialize<T: Encode>(value: T, value_byte_len: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; value_byte_len];
    let written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(written, value_byte_len);
    Ok(bytes)
}

/// Serialize the given encodable value, without error checks.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
fn serialize_unchecked<T: Encode>(value: T, extra_byte_len: usize) -> Vec<u8> {
    serialize(value, size_of::<T>() + extra_byte_len).unwrap()
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

/// Returns a string containing error info.
fn rust_error(details: String) -> *mut ffi::c_void {
    let extra_byte_len = details.len();
    let error = RustMessage {
        status: MessageStatus::Error,
        byte_len: msg_size::<String>(extra_byte_len),
        message: serialize_unchecked(details, extra_byte_len),
    };
    serialize_message(error).unwrap()
}

const fn msg_size<T>(extra_byte_len: usize) -> usize {
    size_of::<RustMessage>() + size_of::<T>() + extra_byte_len
}
