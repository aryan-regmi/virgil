/// Contains APIs to all FFI functions from `libnative.so` (the Rust backend).
library;

import 'dart:ffi';

/// The Rust library for communication.
final _lib = DynamicLibrary.open('libnative.so');

// ==================================================================
// Function types
// ==================================================================

// pub fn load_model(path: *const u8, len: u64)
typedef _LoadModelNativeFn = Void Function(Pointer<Uint8>, Uint64);
typedef _LoadModelFn = void Function(Pointer<Uint8>, int);

// pub fn update_audio_data(audio_data: *const ffi::c_float, len: u64)
typedef _UpdateAudioDataNativeFn = Void Function(Pointer<Float>, Uint64);
typedef _UpdateAudioDataFn = void Function(Pointer<Float>, int);

// pub fn detect_wake_words() -> bool
typedef _DetectWakeWordNativeFn = Bool Function();
typedef _DetectWakeWordFn = bool Function();

// pub fn transcribe(out_len: *mut u64) -> *const u8
typedef _TranscribeNativeFn = Pointer<Uint8> Function(Pointer<Uint64>);
typedef _TranscribeFn = _TranscribeNativeFn;

// pub fn free_transcript(ptr: *mut u8, len: u64)
typedef _FreeTranscriptNativeFn = Void Function(Pointer<Uint8>, Uint64);
typedef _FreeTranscriptFn = void Function(Pointer<Uint8>, int);

// ==================================================================
// Function Bindings
// ==================================================================

/// Loads the Whisper model from the given path.
///
/// @param modelPath The path for the model.
/// @param len The length of the path (in bytes).
final loadModel = _lib.lookupFunction<_LoadModelNativeFn, _LoadModelFn>(
  'load_model',
);

/// Updates the audio data to be transcribed.
///
/// @param audio_data The new audio data.
/// @param len The number of samples in the data.
final updateAudioData = _lib
    .lookupFunction<_UpdateAudioDataNativeFn, _UpdateAudioDataFn>(
      'update_audio_data',
    );

/// Checks if any wake words are present in audio data.
///
/// @returns `true` if a wake word was detected.
///
/// #Note
/// This should only be called **after** [updateAudioData] has been called.
final wakeWordDetected = _lib
    .lookupFunction<_DetectWakeWordNativeFn, _DetectWakeWordFn>(
      'detect_wake_words',
    );

/// Transcribes the audio data into text.
///
/// @param out_len The length of the returned transcription.
///
/// @returns The transcription.
///
/// #Note
/// * This should only be called **after** [updateAudioData] has been called.
/// * [freeTranscript] must be called on the pointer returned by this function.
final transcribe = _lib.lookupFunction<_TranscribeNativeFn, _TranscribeFn>(
  'transcribe',
);

/// Frees the transcript returned by the [transcribe] function.
final freeTranscript = _lib
    .lookupFunction<_FreeTranscriptNativeFn, _FreeTranscriptFn>(
      'free_transcript',
    );
