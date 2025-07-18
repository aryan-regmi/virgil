use std::{
    ptr::slice_from_raw_parts,
    sync::{LazyLock, Mutex},
};

static BYTES: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(|| Mutex::new(vec![]));

#[unsafe(no_mangle)]
pub fn send_bytes(data: *const u8, len: u64) {
    if data.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    let slice = unsafe { slice_from_raw_parts(data, len).as_ref().unwrap() };
    let mut bytes = BYTES.lock().unwrap();
    bytes.clear();
    bytes.extend_from_slice(slice);
}

#[unsafe(no_mangle)]
pub fn get_bytes(out_len: *mut u64) -> *mut u8 {
    let bytes = BYTES.lock().unwrap();
    if out_len.is_null() {
        return std::ptr::null::<u8>().cast_mut();
    }
    unsafe { *out_len = bytes.len() as u64 }

    // Allocate new buffer and copy the data into it
    let mut buf = bytes.clone();
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);

    ptr
}

#[unsafe(no_mangle)]
pub fn free_rust_bytes(ptr: *mut u8, len: u64) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let len = len as usize;
    unsafe {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}
