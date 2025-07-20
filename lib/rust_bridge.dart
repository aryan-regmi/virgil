/// Contains functions and classes responsible for communicating with Rust.
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:logger/logger.dart';
import 'package:virgil/native.dart';

final _logger = Logger();

// FIXME: Send address to pointers between isolates!

// TODO: Use writer/reader pools!

/// Sends the given message to Rust in a background isolate and returns its response.
Future<RustResponse> sendMessage<Message extends BincodeCodable>({
  required MessageType messageType,
  required Message message,
}) async {
  var args = {"messageType": messageType, "message": message};
  return _sendMessage(args);
  // return compute(_sendMessage, args);
}

/// Sends the given message to Rust, and returns its response.
RustResponse _sendMessage<Message extends BincodeCodable>(Map args) {
  final MessageType messageType = args["messageType"];
  final Message message = args["message"];

  // Encode message
  _logger.i('Encoding message...');
  late final int msgType;
  late final Uint8List encodedMessage;
  if (messageType != MessageType.transcribe) {
    msgType = messageType.index;
    encodedMessage = BincodeWriter.encode(message);
  }
  _logger.i('Message encoded');

  // Allocate memory to send to Rust
  final msgLen = encodedMessage.length;
  _logger.w('Message Length: $msgLen');
  final msgPtr = malloc.allocate<Uint8>(msgLen).cast<Void>();
  final responseTypePtr = malloc.allocate<Uint8>(sizeOf<Uint8>());
  final responseLenPtr = malloc.allocate<UintPtr>(sizeOf<UintPtr>());
  final dartAllocs = [msgPtr, responseTypePtr, responseLenPtr];
  _logger.i('Allocated memory to send to Rust');

  // Send to Rust
  _logger.i('Sending message to rust...');
  final responsePtr = sendMessageToRustNative(
    msgType,
    msgPtr,
    msgLen,
    responseTypePtr,
    responseLenPtr,
  );
  _logger.i('Message sent');
  final nativeAllocs = {(responsePtr, responseLenPtr.value)};

  // Decode and return response
  _logger.i('Decoding response...');
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
  _logger.i('Freeing Dart allocations');
  for (var ptr in dartAllocs) {
    malloc.free(ptr);
  }

  _logger.i('Freeing native allocations');
  for (var info in nativeAllocs) {
    freeResponseNative(info.$1, info.$2);
  }
}
