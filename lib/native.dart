/// Contains APIs to all FFI functions from `libnative.so` (the Rust backend).
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';

/// Load the Rust library for communication.
// final _lib = DynamicLibrary.open('libnative.so');
final nativeLib = DynamicLibrary.open(
  'native/target/release/libnative.so',
); // NOTE: FOR LINUX ONLY

// ==================================================================
// Native `Message` types
// ==================================================================

/// The log level for the [nativeLib].
enum LogLevel { trace, debug, info, warn, error }

/// The context used for the [nativeLib].
class Context implements BincodeCodable {
  Context({required this.modelPath, required this.wakeWords});

  Context.empty() : modelPath = '', wakeWords = [];

  /// The path to the `Whisper` model.
  String modelPath;

  /// The list of wake words to listen for/wake to.
  List<String> wakeWords;

  @override
  void decode(BincodeReader reader) {
    modelPath = reader.readString();
    wakeWords = reader.readList(reader.readString);
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(modelPath);
    writer.writeList(wakeWords, writer.writeString);
  }
}

/// The model path to be sent to the [nativeLib];
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

/// The wake words to be sent to the [nativeLib].
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

// fn setup_logs(level: usize)
typedef _SetupLogsNativeFn = Void Function(UintPtr);
typedef _SetupLogsFn = void Function(int);

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

// fn init_dart_api(data: *mut std::ffi::c_void) -> isize
typedef _InitDartApiNativeFn = IntPtr Function(Pointer<Void> data);
typedef _InitDartApiFn = int Function(Pointer<Void> data);

// fn init_dart_port(port: DartPort)
typedef _InitDartPortNativeFn = Void Function(Int64 port);
typedef _InitDartPortFn = void Function(int port);

// fn transcribe_speech(
//   ctx: *mut ffi::c_void,
//   ctx_len: usize,
//   listen_duration_ms: usize
// )
typedef _TranscribeSpeechNativeFn =
    Void Function(Pointer<Void> ctx, UintPtr ctxLen, UintPtr listenDurationMs);
typedef _TranscribeSpeechFn =
    void Function(Pointer<Void> ctx, int ctxLen, int listenDurationMs);

// ==================================================================
// Function Bindings
// ==================================================================

/// Sets up the logging for the Rust library.
final setupLogs = nativeLib.lookupFunction<_SetupLogsNativeFn, _SetupLogsFn>(
  'setup_logs',
);

/// Frees the pointer allocated in Rust.
///
/// @param ptr A Rust pointer.
/// @param len The length of the pointer's contents (in bytes).
final freeRustPtr = nativeLib
    .lookupFunction<_FreeRustPtrNativeFn, _FreeRustPtrFn>('free_rust_ptr');

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
/// The returned pointer must be deallocated using the [freeRustPtr] function.
final initContext = nativeLib
    .lookupFunction<_InitContextNativeFn, _InitContextFn>('init_context');

/// Initalizes the Dart API for FFI communication.
///
/// @param data The native API symbols pointer from Dart.
final initDartApi = nativeLib
    .lookupFunction<_InitDartApiNativeFn, _InitDartApiFn>('init_dart_api');

/// Initalizes the Dart port for FFI communication.
///
/// @param port The receiver port in Dart.
final initDartPort = nativeLib
    .lookupFunction<_InitDartPortNativeFn, _InitDartPortFn>('init_dart_port');

/// Listens continuously to the microphone and transcribes the input if a wake word was detected.
///
/// @param ctx The current context (must be initalized with [initContext]).
/// @param ctxLen The length of the context (in bytes).
/// @param listenDurationMs The number of milliseconds to listen to the microphone.
final transcribeSpeech = nativeLib
    .lookupFunction<_TranscribeSpeechNativeFn, _TranscribeSpeechFn>(
      'transcribe_speech2', // FIXME: Change this to `transcribe_speech`
    );
