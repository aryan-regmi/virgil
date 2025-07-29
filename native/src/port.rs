use std::{
    ffi,
    sync::atomic::{AtomicI64, Ordering},
};

use dart_sys::{
    self, _Dart_CObject, _Dart_CObject__bindgen_ty_1, Dart_CObject,
    Dart_CObject_Type_Dart_CObject_kString, Dart_PostCObject_DL,
};
use tracing::{Level, error, span, trace};

use crate::utils::VirgilResult;

/// Represents a port in Dart.
pub type DartPort = i64;

/// Global atomic to store the Dart SendPort native port.
pub static DART_PORT: AtomicI64 = AtomicI64::new(0);

/// Sets the current port for FFI communication.
pub fn set_dart_port(port: i64) {
    DART_PORT.store(port, Ordering::SeqCst);
}

/// Sends the given string to Dart.
pub fn send_text_to_dart(text: String) -> VirgilResult<()> {
    let span = span!(Level::TRACE, "send_text_to_dart");
    let _enter = span.enter();

    // Create Dart object
    let cstr = ffi::CString::new(text).map_err(|e| error!("{e}")).unwrap();
    let mut dart_obj = Dart_CObject {
        type_: Dart_CObject_Type_Dart_CObject_kString,
        value: _Dart_CObject__bindgen_ty_1 {
            as_string: cstr.as_ptr(),
        },
    };
    trace!("Dart object created");

    // Send object to Dart isolate
    let port = DART_PORT.load(Ordering::SeqCst);
    let success =
        unsafe { Dart_PostCObject_DL.unwrap()(port, &mut dart_obj as *mut _Dart_CObject) };
    if !success {
        error!("Failed to send objet ({:?}) to Dart", dart_obj.type_);
    }
    trace!("Dart object sent to isolate");

    Ok(())
}
