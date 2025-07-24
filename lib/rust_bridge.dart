/// Contains functions and classes responsible for communicating with Rust.
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';
import 'package:ffi/ffi.dart';
import 'package:logger/logger.dart';
import 'package:virgil/native.dart';

final _logger = Logger(level: Level.debug);

// TODO: Use writer/reader pools!

/// Initalizes the Rust context.
Future<Context> initalizeContext({
  required String modelPath,
  required List<String> wakeWords,
}) async {
  // Encode arguments
  final modelPathEncoded = BincodeWriter.encode(ModelPath(path: modelPath));
  final wakeWordsEncoded = BincodeWriter.encode(
    WakeWords(wakeWords: wakeWords),
  );

  // Allocate memory to send to Rust
  final modelPathPtr = calloc.allocate<Uint8>(modelPathEncoded.length);
  final wakeWordsPtr = calloc.allocate<Uint8>(wakeWordsEncoded.length);
  final msgLenOutPtr = calloc.allocate<UintPtr>(sizeOf<UintPtr>());
  final dartAllocs = [
    modelPathPtr,
    wakeWordsPtr,
    msgLenOutPtr,
  ]; // FIXME: Func to free at once

  // Copy encoded message over
  var modelPathBytes = modelPathPtr.asTypedList(modelPathEncoded.length);
  modelPathBytes.setAll(0, modelPathEncoded);
  var wakeWordsBytes = wakeWordsPtr.asTypedList(wakeWordsEncoded.length);
  wakeWordsBytes.setAll(0, wakeWordsEncoded);

  // Create context in Rust
  final msgPtr = initContext(
    modelPathPtr.cast(),
    modelPathBytes.length,
    wakeWordsPtr.cast(),
    wakeWordsBytes.length,
    msgLenOutPtr,
  );
  final nativeAllocs = {
    (msgPtr, msgLenOutPtr.value),
  }; // FIXME: Func to free at once

  // Decode and return response
  final msgBytesPtr = msgPtr.cast<Uint8>();
  final msgBytes = msgBytesPtr.asTypedList(msgLenOutPtr.value);

  final msg = BincodeReader.decode(msgBytes, RustMessage.empty());

  final ctx = BincodeReader.decode(msg.message, Context.empty());

  // Free allocations
  _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);

  return ctx;
}

/// Frees the defined allocations.
void _freeAllocs({
  required List<Pointer> dartAllocs,
  required Set<(Pointer<Void>, int)> nativeAllocs,
}) {
  _logger.d('Freeing Dart allocations');
  for (var ptr in dartAllocs) {
    malloc.free(ptr);
  }

  _logger.d('Freeing native allocations');
  for (var info in nativeAllocs) {
    freeRustPtr(info.$1, info.$2);
  }
}
