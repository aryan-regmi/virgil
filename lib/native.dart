import 'dart:ffi';

/// The Rust library for communication.
final _lib = DynamicLibrary.open('libnative.so');

// ==================================================================
// Function types
// ==================================================================

// pub fn send_bytes(data: *const u8, len: usize)
typedef _SendBytesNativeFn = Void Function(Pointer<Uint8>, Uint64);
typedef _SendBytesFn = void Function(Pointer<Uint8>, int);

// pub fn get_bytes(out_len: *mut u64) -> *mut u8
typedef _GetBytesNativeFn = Pointer<Uint8> Function(Pointer<Uint64>);
typedef _GetBytesFn = _GetBytesNativeFn;

// pub fn free_rust_bytes(ptr: *mut u8, len: u64)
typedef _FreeRustBytesNativeFn = Void Function(Pointer<Uint8>, Uint64);
typedef _FreeRustBytesFn = void Function(Pointer<Uint8>, int);

// ==================================================================
// Function Bindings
// ==================================================================

final sendBytes = _lib.lookupFunction<_SendBytesNativeFn, _SendBytesFn>(
  'send_bytes',
);

final getBytes = _lib.lookupFunction<_GetBytesNativeFn, _GetBytesFn>(
  'get_bytes',
);

final freeRustBytes = _lib
    .lookupFunction<_FreeRustBytesNativeFn, _FreeRustBytesFn>(
      'free_rust_bytes',
    );
