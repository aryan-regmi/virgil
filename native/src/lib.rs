use std::{
    ptr::slice_from_raw_parts,
    sync::{LazyLock, Mutex},
};

use whisper_rs::{WhisperContext, WhisperContextParameters, WhisperState};

/// Represents the library's state.
#[derive(Default)]
struct Model {
    state: Option<WhisperState>,
}

static MODEL: LazyLock<Mutex<Model>> = LazyLock::new(|| Mutex::new(Model::default()));

/// Loads the Whisper model from the given path.
#[unsafe(no_mangle)]
pub fn load_model(path: *const u8, len: u64) {
    if path.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    let slice = unsafe { slice_from_raw_parts(path, len).as_ref().unwrap() };
    let model_path = String::from_utf8(slice.into()).unwrap();

    // Load model
    let ctx =
        WhisperContext::new_with_params(&model_path, WhisperContextParameters::default()).unwrap();
    let state = ctx.create_state().unwrap();
    let mut model = MODEL.lock().unwrap();
    model.state = Some(state);
}

// NOTE: Examples of how to send and receive data from byte buffers.
//
// /// Returns the model path to Dart.
// #[unsafe(no_mangle)]
// pub fn get_model_path(out_len: *mut u64) -> *mut u8 {
//     let path = MODEL_PATH.lock().unwrap();
//     if out_len.is_null() {
//         return std::ptr::null::<u8>().cast_mut();
//     }
//     unsafe { *out_len = path.len() as u64 }
//
//     // Allocate new buffer and copy the data into it
//     let mut buf = path.clone();
//     let ptr = buf.as_mut_ptr();
//     std::mem::forget(buf);
//
//     ptr
// }
//
// /// Frees the memory for returned in `get_model`.
// #[unsafe(no_mangle)]
// pub fn free_model_path(ptr: *mut u8, len: u64) {
//     if ptr.is_null() || len == 0 {
//         return;
//     }
//     let len = len as usize;
//     unsafe {
//         let _ = String::from_raw_parts(ptr, len, len);
//     }
// }
