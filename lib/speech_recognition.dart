import 'dart:async';
import 'dart:collection';
import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:virgil/main.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

final _logger = logger;

class SpeechRecognition {
  // TODO: Add wakeWords, listenDurationMs, and activeListenDuration as parameters!
  SpeechRecognition(LogLevel level) : _level = level;

  /// Max length to listen to the mic for (in milliseconds).
  static const _listenDurationMs = 1000;

  /// The log level of the native library.
  final LogLevel _level;

  /// The context passed to the native library.
  late Context _ctx;

  /// The transcript.
  final LinkedHashSet<String> _transcript = LinkedHashSet();

  /// The processed command.
  String? command;

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

    // Initialize listener
    _receivePort.listen((message) async {
      if (message == null) {
        _logger.e('Invalid message');
        return;
      }
      _transcript.add(message);
    });
  }

  /// Starts listening to the mic and running speech recognition.
  Future<void> startListening() async {
    isListening = true;
    await transcribeMicInput(_ctx, _listenDurationMs);
  }

  /// Stops the microphone.
  Future<void> stopListening() async {
    isListening = false;
    stopMic();
  }

  /// Cleans up resources.
  void dispose() {
    _receivePort.close();
    _transcript.clear();
  }

  Future<void> processCommands() async {
    var sentence = '';
    if (_transcript.isNotEmpty && _transcript.length >= 2) {
      sentence = _transcript.toList().sublist(_transcript.length - 2).join();
    } else if (_transcript.isNotEmpty) {
      sentence = _transcript.last;
    }
    _logger.e(_transcript);

    // var sentence = transcript;
    if (sentence.isNotEmpty) {
      final commandLowercase = sentence.toLowerCase();

      const openStr = 'open';
      final openIdx = commandLowercase.indexOf(openStr);
      if (openIdx >= 0) {
        var cmd = sentence.substring(openIdx + openStr.length).trim();
        if (cmd.isNotEmpty) {
          command = 'Open: $cmd';
        } else {
          command = null;
        }

        // TODO: Actually open!
      }
    }

    // transcript.clear();
  }
}
