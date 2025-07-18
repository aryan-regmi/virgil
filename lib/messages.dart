import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:logger/web.dart';
import 'package:virgil/native.dart';

final _logger = Logger();

/// Sends a string to rust.
Future<void> sendStringToRust(String message) async {
  // Convert String to UTF-8 encoded bytes
  final bytes = Uint8List.fromList(message.codeUnits);

  // Allocate native memory
  final ptr = calloc<Uint8>(bytes.length);
  final nativeBytes = ptr.asTypedList(bytes.length);
  nativeBytes.setAll(0, bytes);

  // Call Rust function
  sendBytes(ptr, bytes.length);
  _logger.i('Sent $message to Rust');

  // Free Dart-allocated memory
  calloc.free(ptr);
}

/// Gets a string from rust.
Future<String> getStringFromRust() async {
  final lenPtr = calloc<Uint64>();
  final dataPtr = getBytes(lenPtr);
  final length = lenPtr.value;
  calloc.free(lenPtr);

  final resultBytes = dataPtr.asTypedList(length);
  final resultString = utf8.decode(resultBytes);
  _logger.i('Received: $resultString from Rust');

  freeRustBytes(dataPtr, length);
  return resultString;
}
