use std::{error::Error, ffi, ptr::slice_from_raw_parts};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::messages::{MessageStatus, RustMessage};

pub type VirgilResult<T> = Result<T, Box<dyn Error>>;

// TODO: Store actual model as `Any`?
//
/// The context passed around for FFI functions.
#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
    pub transcript: String,
}

/// Serialize the given message.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
pub fn serialize_message(value: RustMessage) -> Result<*mut ffi::c_void, String> {
    let byte_len = value.byte_len;
    let bytes = serialize(value, byte_len)?;
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    Ok(response_ptr)
}

/// Serialize the given encodable value.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
pub fn serialize<T: Encode>(value: T, value_byte_len: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; value_byte_len];
    let _written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )
    .map_err(|e| e.to_string())?;
    // assert_eq!(written, value_byte_len);
    Ok(bytes)
}

/// Serialize the given encodable value, without error checks.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
pub fn serialize_unchecked<T: Encode>(value: T, extra_byte_len: usize) -> Vec<u8> {
    serialize(value, size_of::<T>() + extra_byte_len).unwrap()
}

/// Deserialize the value represented by the given pointer and length.
pub fn deserialize<T: Decode<()>>(ptr: *mut ffi::c_void, len: usize) -> Result<T, String> {
    let slice = unsafe {
        let ptr: *mut u8 = ptr.cast();
        let slice = slice_from_raw_parts(ptr, len).as_ref();
        if let Some(slice) = slice {
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

#[allow(dead_code)]
/// Returns a string containing error info.
pub fn rust_error(details: String) -> *mut ffi::c_void {
    let extra_byte_len = details.len();
    let error = RustMessage {
        status: MessageStatus::Error,
        byte_len: msg_size::<String>(extra_byte_len),
        message: serialize_unchecked(details, extra_byte_len),
    };
    serialize_message(error).unwrap()
}

/// Returns the size of a `RustMessage` of type `T`, containing `extra_byte_len` bytes of data.
pub const fn msg_size<T>(extra_byte_len: usize) -> usize {
    size_of::<RustMessage>() + size_of::<T>() + extra_byte_len
}
