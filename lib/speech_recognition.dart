import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_sound/flutter_sound.dart';
import 'package:logger/logger.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

final _logger = Logger(level: Level.info);

// FIXME: Replace `flutter_sound` with `record` and implment low-power & active listening modes.

// TODO: Add ProcessCommands class to actually process the users commands!

/// Runs speech recognition on microphone input and processes the correrponding commands.
class SpeechRecognition {
  SpeechRecognition({
    required this.wakeWords,
    int sampleRate = 16_000,
    int numChannels = 1,
  }) : _sampleRate = sampleRate,
       _numChannels = numChannels {
    _init();
  }

  /// Determines if the microphone is listening.
  bool isListening = false;

  /// The transcribed text.
  String transcript = '';

  /// The wake words to listen for.
  List<String> wakeWords;

  /// The sample rate in `Hz`.
  final int _sampleRate;

  /// The number of audio channels.
  final int _numChannels;

  /// The raw audio data from the microphone.
  final List<double> _accumulatedAudioData = [];

  /// The controller for the audio stream.
  final StreamController<List<Float32List>> _streamController =
      StreamController();

  /// Used to listen to the microphone.
  final FlutterSoundRecorder _recorder = FlutterSoundRecorder();

  /// Initalizes the speech recognition module.
  Future<void> _init() async {
    // Download Whisper model and load it
    final modelManager = await ModelManager.init();
    final modelPath = modelManager.modelPath;
    if (modelPath != null) {
      final response = await sendMessage(
        messageType: MessageType.loadModel,
        message: LoadModelMessage(modelPath: modelPath),
      );
      response.unwrap();
    } else {
      throw Exception('Invalid model path');
    }

    // Request permissions
    var status = await Permission.microphone.request();
    if (status != PermissionStatus.granted) {
      throw RecordingPermissionException('Microphone permissions not granted');
    }

    // Initalize the microphone
    _recorder.openRecorder();

    // Set up wake words
    final setWakeWordsResponse = await sendMessage(
      messageType: MessageType.setWakeWords,
      message: SetWakeWords(wakeWords: wakeWords),
    );
    setWakeWordsResponse.unwrap();

    // Initalize listener that runs speech recognition
    _initSpeechListener();
  }

  /// Starts listening to the microphone.
  Future<void> startListening() async {
    await _recorder.startRecorder(
      codec: Codec.pcmFloat32,
      sampleRate: _sampleRate,
      numChannels: _numChannels,
      audioSource: AudioSource.defaultSource,
      toStreamFloat32: _streamController.sink,
    );
    isListening = true;
  }

  /// Pauses the microphone listener.
  Future<void> pauseListening() async {
    if (isListening) {
      isListening = false;
      await _recorder.stopRecorder();
    }
  }

  /// Closes the microphone listener.
  Future<void> closeListener() async {
    if (isListening) {
      isListening = false;
      await _recorder.stopRecorder();
      await _recorder.closeRecorder();
    }
  }

  /// Restarts the microphone listener after [closeListener] has been called.
  Future<void> restartListener() async {
    if (!isListening) {
      _recorder.openRecorder();
      _initSpeechListener();
      startListening();
    }
  }

  /// Initalizes the speech listener.
  void _initSpeechListener() {
    _streamController.stream.listen((channel) async {
      if (isListening) {
        // TODO: Handle stereo (more than one channel)
        var channelAudio = channel[0];
        _accumulatedAudioData.addAll(channelAudio);
        if (_accumulatedAudioData.length < _sampleRate) {
          return;
        }

        // Update transcript only if wake word is detected
        final detectWakeWordResponse = await sendMessage(
          messageType: MessageType.detectWakeWords,
          message: DetectWakeWordsMessage(
            audioData: Float32List.fromList(_accumulatedAudioData),
          ),
        );
        final WakeWordDetection detectionInfo = detectWakeWordResponse.unwrap();
        if (detectionInfo.detected) {
          // Transcribe audio data if wake word is detected
          _logger.i('Wake word detected');
          final transcribeResponse = await sendMessage(
            messageType: MessageType.transcribe,
            message: TranscribeMessage(
              audioData: channelAudio,
              // audioData: Float32List.fromList(_accumalatedAudioData),
            ),
          );
          final String transcribed = transcribeResponse.unwrap();
          transcript = transcribed;

          // Remove detected wake word from transcript
          if (detectionInfo.endIdx != 0) {
            transcript = transcribed.replaceRange(
              detectionInfo.startIdx,
              detectionInfo.endIdx,
              '',
            );
          } else {
            transcript = transcribed;
          }
        }

        _accumulatedAudioData.clear();
      }
    });
  }
}
