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
Future<RustResponse> sendMessage<Message extends DartMessage>({
  required MessageType messageType,
  required Message message,
}) async {
  var args = {"messageType": messageType, "message": message};
  return _sendMessage(args);
  // return compute(_sendMessage, args);
}

/// Sends the given message to Rust, and returns its response.
RustResponse _sendMessage<Message extends DartMessage>(Map args) {
  final MessageType messageType = args["messageType"];
  final Message message = args["message"];

  // Encode message
  late final int msgType = messageType.index;
  late final Uint8List encodedMessage;
  if (messageType == MessageType.transcribe) {
    encodedMessage = Uint8List(0); // Ignore ZST messages
  } else {
    encodedMessage = BincodeWriter.encode(message);
  }
  _logger.i('Message encoded: $message');

  // Allocate memory to send to Rust
  final msgLen = encodedMessage.length;
  final msgPtr = calloc.allocate<Uint8>(msgLen);
  final responseTypePtr = calloc.allocate<Uint8>(sizeOf<Uint8>());
  final responseLenPtr = calloc.allocate<UintPtr>(sizeOf<UintPtr>());
  final dartAllocs = [msgPtr, responseTypePtr, responseLenPtr];
  _logger.i('Allocated memory for Rust methods');

  // Copy encoded message over
  var msgBytes = msgPtr.asTypedList(msgLen);
  msgBytes.setAll(0, encodedMessage);

  // Send to Rust
  final responsePtr = sendMessageToRustNative(
    msgType,
    msgPtr.cast(),
    msgLen,
    responseTypePtr,
    responseLenPtr,
  );
  final nativeAllocs = {(responsePtr, responseLenPtr.value)};
  _logger.i('Message sent to Rust');

  // Validate response
  if (responsePtr.address == nullptr.address) {
    _logger.e('Response from Rust was NULL');
    return ErrorResponse('Invalid response from Rust');
  }

  // Decode and return response
  _logger.i('Decoding response...');
  final responseBytesPtr = responsePtr.cast<Uint8>();
  final responseBytes = responseBytesPtr.asTypedList(responseLenPtr.value);
  final responseType = ResponseType.values[responseTypePtr.value];
  final response = _decodeResponse(responseType, responseBytes);
  _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);
  return response;
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

/// Decodes the response into the correct types.
RustResponse _decodeResponse(ResponseType responseType, Uint8List bytes) {
  switch (responseType) {
    case ResponseType.text:
      final response = BincodeReader.decode(bytes, TextResponse.empty());
      _logger.i('Decoded response: TextResponse(${response.text})');
      return response;
    case ResponseType.wakeWord:
      final response = BincodeReader.decode(bytes, WakeWordResponse.empty());
      _logger.i(
        'Decoded response: WakeWordResponse {\n\tdetected: ${response.detection.detected},\n\tstartIdx: ${response.detection.startIdx},\n\tendIdx: ${response.detection.endIdx} }',
      );
      return response;
    case ResponseType.error:
      final response = BincodeReader.decode(bytes, ErrorResponse.empty());
      _logger.i('Decoded response: ErrorResponse(${response.text})');
      return response;
  }
}
