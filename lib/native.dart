/// Contains APIs to all FFI functions from `libnative.so` (the Rust backend).
library;

import 'dart:ffi';

/// The Rust library for communication.
final _lib = DynamicLibrary.open('libnative.so');

// ==================================================================
// Function types
// ==================================================================

// send_message_to_rust(msg_ptr: *const ffi::c_void, msg_len: usize, resp_len: *mut usize) -> *mut ffi::c_void
typedef _SendMessageToRustNativeFn =
    Pointer<Void> Function(
      Pointer<Void> msgPtr,
      UintPtr msgLen,
      Pointer<UintPtr> resLenOut,
    );
typedef _SendMessageToRustFn =
    Pointer<Void> Function(
      Pointer<Void> msgPtr,
      int msgLen,
      Pointer<UintPtr> resLenOut,
    );

// free_response(ptr: *const ffi::c_void, len: usize)
typedef _FreeResponseNativeFn = Void Function(Pointer<Void> ptr, UintPtr len);
typedef _FreeResponseFn = void Function(Pointer<Void> ptr, int len);

// ==================================================================
// Function Bindings
// ==================================================================

/// Sends the specified message *to* Rust *from* Dart.
///
/// @param modelPath The path for the model.
/// @param len The length of the path (in bytes).
///
/// @returns A pointer to a `Response` object.
///
/// # Note
/// The returned pointer must be deallocated using the [freeResponseNative] function.
final sendMessageToRustNative = _lib
    .lookupFunction<_SendMessageToRustNativeFn, _SendMessageToRustFn>(
      'send_message_to_rust',
    );

/// Frees the response returned by the [sendMessageToRustNative] function.
///
/// @param ptr A pointer to the Rust `Response`.
/// @param len The length of the response (in bytes).
final freeResponseNative = _lib
    .lookupFunction<_FreeResponseNativeFn, _FreeResponseFn>('free_response');
