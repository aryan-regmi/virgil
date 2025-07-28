/// Contains functions and classes responsible for communicating with Rust.
library;

import 'dart:ffi';

import 'package:d_bincode/d_bincode.dart';
import 'package:ffi/ffi.dart';
import 'package:logger/logger.dart';
import 'package:virgil/native.dart';

final _logger = Logger(level: Level.debug);

// TODO: Use writer/reader pools!

// TODO: Add log messages

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
  final ctxLenOutPtr = calloc.allocate<UintPtr>(sizeOf<UintPtr>());
  final dartAllocs = [modelPathPtr, wakeWordsPtr, ctxLenOutPtr];

  // Copy encoded message over
  var modelPathBytes = modelPathPtr.asTypedList(modelPathEncoded.length);
  modelPathBytes.setAll(0, modelPathEncoded);
  var wakeWordsBytes = wakeWordsPtr.asTypedList(wakeWordsEncoded.length);
  wakeWordsBytes.setAll(0, wakeWordsEncoded);

  // Call Rust func to create pointer
  final ctxPtr = initContext(
    modelPathPtr.cast(),
    modelPathBytes.length,
    wakeWordsPtr.cast(),
    wakeWordsBytes.length,
    ctxLenOutPtr,
  );
  final nativeAllocs = {(ctxPtr, ctxLenOutPtr.value)};

  // Decode and return response
  final ctxBytesPtr = ctxPtr.cast<Uint8>();
  final ctxBytes = ctxBytesPtr.asTypedList(ctxLenOutPtr.value);
  final ctx = BincodeReader.decode(ctxBytes, Context.empty());

  // Free allocations
  _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: nativeAllocs);

  _logger.i('Context initalized');
  return ctx;
}

Future<void> listenToMic(Context ctx, int listenDurationMs) async {
  // Encode arguments
  final ctxEncoded = BincodeWriter.encode(ctx);

  // Allocate memory to send to Rust
  final ctxPtr = calloc.allocate<Uint8>(ctxEncoded.length);
  final dartAllocs = [ctxPtr];

  // Copy encoded message over
  var ctxBytes = ctxPtr.asTypedList(ctxEncoded.length);
  ctxBytes.setAll(0, ctxEncoded);

  // Call Rust function
  listen(ctxPtr.cast(), ctxEncoded.length, listenDurationMs);

  // Free allocations
  _freeAllocs(dartAllocs: dartAllocs, nativeAllocs: {});
}

/// Frees the defined allocations.
void _freeAllocs({
  required List<Pointer> dartAllocs,
  required Set<(Pointer<Void>, int)> nativeAllocs,
}) {
  for (var ptr in dartAllocs) {
    malloc.free(ptr);
  }
  _logger.i('Dart allocations freed');

  for (var info in nativeAllocs) {
    freeRustPtr(info.$1, info.$2);
  }
  _logger.i('Native allocations freed');
}
