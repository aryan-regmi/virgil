import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:logger/web.dart';
import 'package:virgil/native.dart';

final _logger = Logger();

/// Sends the Whisper model path to Rust to load.
Future<void> sendModelPathToRust(String path) async {
  // Convert `path` to UTF-8 encoded bytes
  final bytes = Uint8List.fromList(utf8.encode(path));

  // Allocate native memory
  final ptr = calloc<Uint8>(bytes.length);
  final nativeBytes = ptr.asTypedList(bytes.length);
  nativeBytes.setAll(0, bytes);

  // Load the model in Rust
  loadModel(ptr, bytes.length);
  _logger.i('Model path: $path (Dart -> Rust)');

  // Free Dart-allocated memory
  calloc.free(ptr);
}
