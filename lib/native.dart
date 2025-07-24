/// Contains APIs to all FFI functions from `libnative.so` (the Rust backend).
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';

/// The Rust library for communication.
// final _lib = DynamicLibrary.open('libnative.so');
final _lib = DynamicLibrary.open(
  'native/target/release/libnative.so',
); // NOTE: FOR LINUX ONLY

// ==================================================================
// Native `Message` types
// ==================================================================

class Context implements BincodeCodable {
  Context({
    required this.modelPath,
    required this.wakeWords,
    required this.transcript,
  });

  Context.empty() : modelPath = '', wakeWords = [], transcript = '';

  String modelPath;
  List<String> wakeWords;
  String transcript;

  @override
  void decode(BincodeReader reader) {
    modelPath = reader.readString();
    wakeWords = reader.readList(reader.readString);
    transcript = reader.readString();
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(modelPath);
    writer.writeList(wakeWords, writer.writeString);
    writer.writeString(transcript);
  }
}

class ModelPath implements BincodeCodable {
  ModelPath({required this.path});

  ModelPath.empty() : path = '';

  String path;

  @override
  void decode(BincodeReader reader) {
    path = reader.readString();
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(path);
  }
}

class WakeWords implements BincodeCodable {
  WakeWords({required this.wakeWords});

  WakeWords.empty() : wakeWords = [];

  List<String> wakeWords;

  @override
  void decode(BincodeReader reader) {
    wakeWords = reader.readList(reader.readString);
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeList(wakeWords, writer.writeString);
  }
}

// ==================================================================
// Function types
// ==================================================================

// fn free_rust_ptr(ptr: *const ffi::c_void, len: usize)
typedef _FreeRustPtrNativeFn = Void Function(Pointer<Void> ptr, UintPtr len);
typedef _FreeRustPtrFn = void Function(Pointer<Void> ptr, int len);

// fn init_context(
//     model_path: *mut ffi::c_void,
//     model_path_len: usize,
//     wake_words: *mut ffi::c_void,
//     wake_words_len: usize,
//     ctx_len_out: *mut usize,
// ) -> *mut ffi::c_void {
typedef _InitContextNativeFn =
    Pointer<Void> Function(
      Pointer<Void> modelPath,
      UintPtr modelPathLen,
      Pointer<Void> wakeWords,
      UintPtr wakeWordsLen,
      Pointer<UintPtr> ctxLenOut,
    );
typedef _InitContextFn =
    Pointer<Void> Function(
      Pointer<Void> modelPath,
      int modelPathLen,
      Pointer<Void> wakeWords,
      int wakeWordsLen,
      Pointer<UintPtr> ctxLenOut,
    );

// fn listen_for_wake_words(ctx: *mut ffi::c_void, ctx_len: usize, miliseconds: usize) -> bool
typedef _ListenForWakeWordNativeFn =
    Bool Function(Pointer<Void> ctx, UintPtr ctxLen, UintPtr miliseconds);
typedef _ListenForWakeWordFn =
    bool Function(Pointer<Void> ctx, int ctxLen, int miliseconds);

// fn listen_for_commands(
//     ctx: *mut ffi::c_void,
//     ctx_len: usize,
//     miliseconds: usize,
//     ctx_len_out: *mut usize,
// ) -> *mut ffi::c_void
typedef _ListenForCommandsNativeFn =
    Pointer<Void> Function(
      Pointer<Void> ctx,
      UintPtr ctxLen,
      UintPtr miliseconds,
      Pointer<UintPtr> ctxLenOut,
    );
typedef _ListenForCommandsFn =
    Pointer<Void> Function(
      Pointer<Void> ctx,
      int ctxLen,
      int miliseconds,
      Pointer<UintPtr> ctxLenOut,
    );

// ==================================================================
// Function Bindings
// ==================================================================

/// Frees the pointer allocated in Rust.
///
/// @param ptr A Rust pointer.
/// @param len The length of the pointer's contents (in bytes).
final freeRustPtr = _lib.lookupFunction<_FreeRustPtrNativeFn, _FreeRustPtrFn>(
  'free_rust_ptr',
);

/// Initalizes the application context.
///
/// @param modelPath The path of the `Whisper` model.
/// @param modelPathLen The length of the model path (in bytes).
/// @param wakeWords A list of wake words.
/// @param wakeWordsLen The length of the wake words (in bytes).
///
/// @returns A pointer to a `Context` object.
///
/// # Note
/// The returned pointer must be deallocated using the [freeResponseNative] function.
final initContext = _lib.lookupFunction<_InitContextNativeFn, _InitContextFn>(
  'init_context',
);

/// Listens for wake words.
///
/// @param ctx The application `Context`.
/// @param ctxLen The length of the context (in bytes).
/// @param miliSeconds The number of miliseconds to listen for.
///
/// @returns `true` if wake word was detected.
final listenForWakeWords = _lib
    .lookupFunction<_ListenForWakeWordNativeFn, _ListenForWakeWordFn>(
      'listen_for_wake_words',
    );

/// Listens for commands.
///
/// @param ctx The application `Context`.
/// @param ctxLen The length of the context (in bytes).
/// @param miliSeconds The number of miliseconds to listen for.
///
/// @returns `true` if wake word was detected.
///
/// #Note
/// This should be called **after** [listenForWakeWords] returns `true`.
final listenForCommands = _lib
    .lookupFunction<_ListenForCommandsNativeFn, _ListenForCommandsFn>(
      'listen_for_commands',
    );
