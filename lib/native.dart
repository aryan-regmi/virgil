import 'dart:ffi';


/// The Rust library for communication.
final _lib = DynamicLibrary.open('libnative.so');

// ==================================================================
// Function types
// ==================================================================

// pub fn load_model(path: *const u8, len: u64)
typedef _LoadModelNativeFn = Void Function(Pointer<Uint8>, Uint64);
typedef _LoadModelFn = void Function(Pointer<Uint8>, int);

// ==================================================================
// Function Bindings
// ==================================================================

final loadModel = _lib.lookupFunction<_LoadModelNativeFn, _LoadModelFn>(
  'load_model',
);

