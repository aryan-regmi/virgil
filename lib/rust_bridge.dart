/// Contains functions and classes responsible for communicating with Rust.
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:virgil/native.dart';

// TODO: Use writer/reader pools!

/// Sends the given message to Rust in a background isolate and returns its response.
Future<RustResponse> sendMessage<Message extends BincodeCodable>({
  required MessageType messageType,
  required Message message,
}) async {
  var args = {"messageType": messageType, "message": message};
  return compute(_sendMessage, args);
}

/// Sends the given message to Rust, and returns its response.
RustResponse _sendMessage<Message extends BincodeCodable>(Map args) {
  final MessageType messageType = args["messageType"];
  final Message message = args["message"];

  // Encode message
  late final int msgType;
  late final Uint8List encodedMessage;
  if (messageType != MessageType.transcribe) {
    msgType = messageType.index;
    encodedMessage = BincodeWriter.encode(message);
  }

  // Allocate memory to send to Rust
  final msgLen = encodedMessage.length;
  final msgPtr = malloc<Uint8>(msgLen).cast<Void>();
  final responseTypePtr = malloc<Uint8>();
  final responseLenPtr = malloc<UintPtr>();
  final dartAllocs = [msgPtr, responseTypePtr, responseLenPtr];

  // Send to Rust
  final responsePtr = sendMessageToRustNative(
    msgType,
    msgPtr,
    msgLen,
    responseTypePtr,
    responseLenPtr,
  );
  final nativeAllocs = {(responsePtr, responseLenPtr.value)};

  // Decode and return response
  final responseBytesPtr = responsePtr.cast<Uint8>();
  final responseBytes = responseBytesPtr.asTypedList(responseLenPtr.value);
  final responseType = ResponseType.values[responseTypePtr.value];
  switch (responseType) {
    case ResponseType.text:
      final response = BincodeReader.decode(
        responseBytes,
        TextResponse.empty(),
      );
      _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);
      return response;
    case ResponseType.wakeWord:
      final response = BincodeReader.decode(
        responseBytes,
        WakeWordResponse.empty(),
      );
      _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);
      return response;
    case ResponseType.error:
      final response = BincodeReader.decode(
        responseBytes,
        ErrorResponse.empty(),
      );
      _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);
      return response;
  }
}

/// Frees the defined allocations.
void _freeAllocs({
  required List<Pointer> dartAllocs,
  required Set<(Pointer<Void>, int)> nativeAllocs,
}) {
  for (var ptr in dartAllocs) {
    malloc.free(ptr);
  }
  for (var info in nativeAllocs) {
    freeResponseNative(info.$1, info.$2);
  }
}
