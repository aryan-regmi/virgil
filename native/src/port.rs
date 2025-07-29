use std::{
    ffi,
    sync::atomic::{AtomicI64, Ordering},
};

use tracing::{Level, error, info, span, trace};

use crate::api::DART_POST_FN;

pub type DartPort = i64;

#[repr(C)]
pub enum DartCObjectType {
    String = 8,
    // Null = 0,
    // Bool = 1,
    // Int32 = 2,
    // Int64 = 3,
    // Double = 4,
}

#[repr(C)]
pub union DartCObjectValue {
    as_int64: i64,
    as_string: *const ffi::c_char,
}

#[repr(C)]
pub struct DartCObject {
    type_: DartCObjectType,
    value: DartCObjectValue,
}

// Global atomic to store the Dart SendPort native port.
pub static DART_PORT: AtomicI64 = AtomicI64::new(0);

pub fn send_text_to_dart(text: String) {
    let span = span!(Level::TRACE, "send_text_to_dart");
    let _enter = span.enter();

    // Create Dart object
    let cstr = ffi::CString::new(text).map_err(|e| error!("{e}")).unwrap();
    let mut dart_obj = DartCObject {
        type_: DartCObjectType::String,
        value: DartCObjectValue {
            as_string: cstr.as_ptr(),
        },
    };
    info!("Dart object created");

    // Send object to Dart isolate
    let port = DART_PORT.load(Ordering::SeqCst);
    let func = DART_POST_FN.lock().unwrap().unwrap();
    let result = func(port, &mut dart_obj);
    trace!("Result: {result}");
    info!("Dart object sent to isolate");

    // std::thread::sleep(std::time::Duration::from_millis(1));
    // std::mem::drop(cstr);
}
