import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:virgil/main.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

final _logger = logger;

class SpeechRecognition {
  SpeechRecognition(LogLevel level) : _level = level;

  /// The log level of the native library.
  final LogLevel _level;

  /// The context passed to the native library.
  late Context _ctx;

  /// The transcript.
  String _transcript = _defaultTranscript;

  /// Default transcript message.
  static const _defaultTranscript = 'Waiting...';

  /// The port used for FFI communications.
  final _receivePort = ReceivePort();

  /// Determines if the mic is listening.
  bool isListening = false;

  void init() async {
    // Setup logs
    setupLogs(_level.index);

    // Initialize FFI
    await initFFI(_receivePort.sendPort.nativePort);

    // Download and Initialize model
    final modelManager = await ModelManager.init();
    if (modelManager.modelPath != null) {
      _ctx = await initalizeContext(
        modelPath: modelManager.modelPath!,
        wakeWords: ['Wake', 'Test'],
      );
    } else {
      throw Exception('Failed to initalize Whisper model');
    }
  }

  /// Returns a stream builder on the native `ReceivePort` stream.
  StreamBuilder<dynamic> streamBuilder() {
    return StreamBuilder(
      stream: _receivePort,
      builder: (ctx, snapshot) {
        if (isListening && snapshot.hasData) {
          String? message = snapshot.data;
          if (message == null) {
            _logger.e('Invalid message');
            return Text('Invalid message');
          }
          _transcript = message;
          return Text(_transcript);
        }
        return Text(_defaultTranscript);
      },
    );
  }

  // TODO: Actually process commands!
  //
  /// Starts listening to the mic and running speech recognition.
  Future<void> startListening() async {
    isListening = true;
    await transcribeMicInput(_ctx, 1000);
  }

  /// Stops the microphone.
  Future<void> stopListening() async {
    isListening = false;
    stopMic();
  }

  /// Cleans up resources.
  void dispose() {
    _receivePort.close();
  }
}
