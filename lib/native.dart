/// Contains APIs to all FFI functions from `libnative.so` (the Rust backend).
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';
import 'package:flutter/foundation.dart';

/// The Rust library for communication.
final _lib = DynamicLibrary.open('libnative.so');

// ==================================================================
// Native `Message` types
// ==================================================================

// NOTE: This must be kept in sync with Rust's `MessageType`.
//
/// The message type sent to Rust.
enum MessageType {
  loadModel,
  updateAudioData,
  detectWakeWords,
  transcribe,
  debug,
}

class LoadModelMessage implements BincodeCodable {
  LoadModelMessage({required this.modelPath});

  String modelPath;

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(modelPath);
  }

  @override
  void decode(BincodeReader reader) {
    modelPath = reader.readString();
  }
}

class UpdateAudioDataMessage implements BincodeCodable {
  UpdateAudioDataMessage({required this.audioData});

  Float32List audioData;

  @override
  void encode(BincodeWriter writer) {
    writer.writeFloat32List(audioData);
  }

  @override
  void decode(BincodeReader reader) {
    audioData = Float32List.fromList(reader.readFloat32List());
  }
}

class DetectWakeWordsMessage implements BincodeCodable {
  DetectWakeWordsMessage({required this.wakeWords});

  List<String> wakeWords;

  @override
  void encode(BincodeWriter writer) {
    writer.writeList(wakeWords, writer.writeString);
  }

  @override
  void decode(BincodeReader reader) {
    wakeWords = reader.readList(reader.readString);
  }
}

// NOTE: This is a ZST in Rust, so no need to send it across.
class TranscribeMessage implements BincodeCodable {
  @override
  void decode(BincodeReader reader) {}

  @override
  void encode(BincodeWriter writer) {}
}

class DebugMessage implements BincodeCodable {
  DebugMessage({required this.message});

  String message;

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(message);
  }

  @override
  void decode(BincodeReader reader) {
    message = reader.readString();
  }
}

// ==================================================================
// Native `Response` types
// ==================================================================

// NOTE: This must be kept in sync with Rust's `ResponseType`.
//
/// The response type sent from Rust.
enum ResponseType { text, wakeWord, error }

abstract class RustResponse<T> {
  ResponseType get kind;
  T get value;
}

class TextResponse extends RustResponse<String> implements BincodeCodable {
  String text;

  TextResponse(this.text);
  TextResponse.empty() : text = '';

  @override
  void decode(BincodeReader reader) {
    text = reader.readString();
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(text);
  }

  @override
  ResponseType get kind => ResponseType.text;

  @override
  String get value => text;
}

class WakeWordDetection implements BincodeCodable {
  bool detected;
  int? startIdx;
  int? endIdx;

  WakeWordDetection(this.detected);
  WakeWordDetection.empty() : detected = false;

  @override
  void decode(BincodeReader reader) {
    detected = reader.readBool();
    startIdx = reader.readOptionU64();
    endIdx = reader.readOptionU64();
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeBool(detected);
    writer.writeOptionU64(startIdx);
    writer.writeOptionU64(endIdx);
  }
}

class WakeWordResponse extends RustResponse<WakeWordDetection>
    implements BincodeCodable {
  WakeWordDetection detection;

  WakeWordResponse(this.detection);
  WakeWordResponse.empty() : detection = WakeWordDetection.empty();

  @override
  void decode(BincodeReader reader) {
    detection = reader.readNestedObjectForFixed(WakeWordDetection.empty());
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeNestedValueForFixed(detection);
  }

  @override
  ResponseType get kind => ResponseType.wakeWord;

  @override
  WakeWordDetection get value => detection;
}

class ErrorResponse extends RustResponse<String> implements BincodeCodable {
  String text;

  ErrorResponse(this.text);
  ErrorResponse.empty() : text = '';

  @override
  void decode(BincodeReader reader) {
    text = reader.readString();
  }

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(text);
  }

  @override
  ResponseType get kind => ResponseType.error;

  @override
  String get value => text;
}

// ==================================================================
// Function types
// ==================================================================

// fn send_message_to_rust(
//     msg_type: u8,
//     msg_ptr: *const ffi::c_void,
//     msg_len: usize,
//     resp_type: *mut u8,
//     resp_len: *mut usize,
// ) -> *mut ffi::c_void
typedef _SendMessageToRustNativeFn =
    Pointer<Void> Function(
      Uint8 msgType,
      Pointer<Void> msgPtr,
      UintPtr msgLen,
      Pointer<Uint8> respTypeOut,
      Pointer<UintPtr> respLenOut,
    );
typedef _SendMessageToRustFn =
    Pointer<Void> Function(
      int msgType,
      Pointer<Void> msgPtr,
      int msgLen,
      Pointer<Uint8> respTypeOut,
      Pointer<UintPtr> respLenOut,
    );

// fn free_response(ptr: *const ffi::c_void, len: usize)
typedef _FreeResponseNativeFn = Void Function(Pointer<Void> ptr, UintPtr len);
typedef _FreeResponseFn = void Function(Pointer<Void> ptr, int len);

// ==================================================================
// Function Bindings
// ==================================================================

/// Sends the specified message *to* Rust *from* Dart.
///
/// @param msgType The type of the message.
/// @param msgPtr The actual message.
/// @param msgLen The length of the message (in bytes).
/// @param respTypeOut The type of the response from Rust.
/// @param respLenOut The length of the response (in bytes).
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
