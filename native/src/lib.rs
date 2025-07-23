use std::{ffi, ptr::slice_from_raw_parts_mut};

use bincode::{Encode, encode_into_slice};

use crate::messages::{Message, Transcript};

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

/// Initalizes the transcript.
#[unsafe(no_mangle)]
pub fn init_transcript(capacity: usize, len_out: *mut usize) -> *mut ffi::c_void {
    let transcript = Transcript(String::with_capacity(capacity));
    return serialize(transcript, len_out).expect("Unable to serialize");
}

/// Initalizes the microphone listener.
#[unsafe(no_mangle)]
pub fn init_listener() {}

/// Serialize the given value.
///
/// # Note
/// The caller must free the the returned pointer with [free_response].
fn serialize<T: Message>(value: T, value_len_out: *mut usize) -> Result<*mut ffi::c_void, String> {
    let mut bytes = vec![0; value.byte_len()];
    let written = encode_into_slice(
        value,
        bytes.as_mut_slice(),
        bincode::config::standard().with_fixed_int_encoding(),
    )
    .map_err(|e| e.to_string())?;
    unsafe { *value_len_out = written };
    let response_ptr: *mut ffi::c_void = Box::into_raw(bytes.into_boxed_slice()).cast();
    Ok(response_ptr)
}

/// Returns a string containing error info.
fn rust_error(details: String) -> *mut ffi::c_void {
    todo!()
}
