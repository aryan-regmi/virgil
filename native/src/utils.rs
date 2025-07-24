use std::{error::Error, ffi, ptr::slice_from_raw_parts};

use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};

use crate::messages::Message;

pub type VirgilResult<T> = Result<T, Box<dyn Error>>;

/// The context passed around for FFI functions.
#[derive(Encode, Decode)]
pub struct Context {
    pub model_path: String,
    pub wake_words: Vec<String>,
    pub transcript: String,
}

/// Serialize the given encodable value.
///
/// # Note
/// The caller must free the the returned pointer with [free_rust_ptr].
pub fn serialize<T: Message>(
    value: T,
    value_len_out: *mut usize,
) -> Result<*mut ffi::c_void, String> {
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
