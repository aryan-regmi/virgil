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
enum MessageType implements BincodeEncodable {
  loadModel,
  updateAudioData,
  detectWakeWords,
  transcribe;

  @override
  void encode(BincodeWriter writer) {
    switch (this) {
      case MessageType.loadModel:
        writer.writeU8(0);
        break;
      case MessageType.updateAudioData:
        writer.writeU8(1);
        break;
      case MessageType.detectWakeWords:
        writer.writeU8(2);
        break;
      case MessageType.transcribe:
        writer.writeU8(3);
        break;
    }
  }
}

class LoadModelMessage implements BincodeEncodable {
  LoadModelMessage({required this.modelPath});

  final String modelPath;

  @override
  void encode(BincodeWriter writer) {
    writer.writeString(modelPath);
  }
}

class UpdateAudioDataMessage implements BincodeEncodable {
  UpdateAudioDataMessage({required this.audioData});

  final Float32List audioData;

  @override
  void encode(BincodeWriter writer) {
    writer.writeFloat32List(audioData);
  }
}

class DetectWakeWordsMessage implements BincodeEncodable {
  DetectWakeWordsMessage({required this.wakeWords});

  final List<String> wakeWords;

  @override
  void encode(BincodeWriter writer) {
    writer.writeList(wakeWords, writer.writeString);
  }
}

// NOTE: This is a ZST in Rust, so no need to send it across.
class TranscribeMessage {}

// ==================================================================
// Native `Response` types
// ==================================================================

// NOTE: This must be kept in sync with Rust's `MessageType`.
//
/// The response type sent from Rust.
enum ResponseType implements BincodeEncodable {
  loadModel,
  updateAudioData,
  detectWakeWords,
  transcribe;

  @override
  void encode(BincodeWriter writer) {
    switch (this) {
      case ResponseType.loadModel:
        writer.writeU8(loadModel.index);
        break;
      case ResponseType.updateAudioData:
        writer.writeU8(updateAudioData.index);
        break;
      case ResponseType.detectWakeWords:
        writer.writeU8(detectWakeWords.index);
        break;
      case ResponseType.transcribe:
        writer.writeU8(transcribe.index);
        break;
    }
  }
}

// ==================================================================
// Function types
// ==================================================================

// fn send_message_to_rust(
//     msg_type: u8,
//     msg_ptr: *const ffi::c_void,
//     msg_len: usize,
//     resp_len: *mut usize,
// ) -> *mut ffi::c_void
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

// fn free_response(ptr: *const ffi::c_void, len: usize)
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
