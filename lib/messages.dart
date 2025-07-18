/// Contains messages passed between Rust and Flutter.
library;

import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:logger/web.dart';
import 'package:virgil/native.dart';

final _logger = Logger();

/// Sends the Whisper model path to Rust, which then loads the model.
Future<void> sendModelPathAndLoadModel(String path) async {
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

/// Sends audio data to Rust.
Future<void> sendAudioData(Float32List audioData) async {
  // Create native float array and copy audioData over
  final ptr = calloc<Float>(audioData.length);
  final nativeBytes = ptr.asTypedList(audioData.length);
  nativeBytes.setAll(0, audioData);

  // Send audio data to Rust
  updateAudioData(ptr, audioData.length);
  _logger.i('Audio Data: [${audioData.length} samples] (Dart -> Rust)');

  // Free Dart-allocated memory
  calloc.free(ptr);
}
