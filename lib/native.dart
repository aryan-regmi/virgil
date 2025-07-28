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

// fn setup_logs()
typedef _SetupLogsNativeFn = Void Function();
typedef _SetupLogsFn = void Function();

// fn free_rust_ptr(ptr: *const ffi::c_void, len: usize)
typedef _FreeRustPtrNativeFn = Void Function(Pointer<Void> ptr, UintPtr len);
typedef _FreeRustPtrFn = void Function(Pointer<Void> ptr, int len);

// fn init_context(
//     model_path: *mut ffi::c_void,
//     model_path_len: usize,
//     wake_words: *mut ffi::c_void,
//     wake_words_len: usize,
//     ctx_len_out: *mut usize,
// ) -> *mut ffi::c_void
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

// fn transcribe_speech(
//     ctx: *mut ffi::c_void,
//     ctx_len: usize,
//     listen_duration_ms: usize,
//     mut _ctx_out: *mut ffi::c_void,
//     ctx_len_out: *mut usize,
// )
typedef _TranscribeSpeechNativeFn =
    Pointer<Void> Function(
      Pointer<Void> ctx,
      UintPtr ctxLen,
      UintPtr listenDurationMs,
      Pointer<Void> ctxOut,
      Pointer<UintPtr> ctxLenOut,
    );
typedef _TranscribeSpeechFn =
    Pointer<Void> Function(
      Pointer<Void> ctx,
      int ctxLen,
      int timeoutMs,
      Pointer<Void> ctxOut,
      Pointer<UintPtr> ctxLenOut,
    );

// ==================================================================
// Function Bindings
// ==================================================================

/// Sets up the logging for the Rust library.
final setupLogs = _lib.lookupFunction<_SetupLogsNativeFn, _SetupLogsFn>(
  'setup_logs',
);

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
/// @param ctxLenOut The length of the returned context (in bytes).
///
/// @returns A pointer to the initalized `Context` object.
///
/// # Note
/// The returned pointer must be deallocated using the [freeResponseNative] function.
final initContext = _lib.lookupFunction<_InitContextNativeFn, _InitContextFn>(
  'init_context',
);

// fn transcribe_speech(
//     ctx: *mut ffi::c_void,
//     ctx_len: usize,
//     listen_duration_ms: usize,
//     mut _ctx_out: *mut ffi::c_void,
//     ctx_len_out: *mut usize,
// )

/// Listens continuously to the microphone and transcribes the input if a wake word was detected.
///
/// @param ctx The current context (must be initalized with [initContext]).
/// @param ctxLen The length of the context (in bytes).
/// @param listenDurationMs The number of milliseconds to listen to the microphone.
/// @param ctxOut The context with the updated transcript, returned by the function.
/// @param ctxLenOut The length of the returned `ctxOut` (in bytes).
final transcribeSpeech = _lib
    .lookupFunction<_TranscribeSpeechNativeFn, _TranscribeSpeechFn>(
      'transcribe_speech',
    );
