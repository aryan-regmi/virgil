import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_sound/flutter_sound.dart';
import 'package:logger/logger.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

final _logger = Logger();

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
  String? transcript;

  /// The wake words to listen for.
  List<String> wakeWords;

  /// The sample rate in `Hz`.
  final int _sampleRate;

  /// The number of audio channels.
  final int _numChannels;

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
    _streamController.stream.listen((channel) async {
      if (isListening) {
        // TODO: Handle stereo (more than one channel)
        var monoAudio = channel[0];
        final updateAudioResponse = await sendMessage(
          messageType: MessageType.updateAudioData,
          message: UpdateAudioDataMessage(audioData: monoAudio),
        );
        updateAudioResponse.unwrap();

        // Update transcript only if wake word is detected
        final detectWakeWordResponse = await sendMessage(
          messageType: MessageType.detectWakeWords,
          message: DetectWakeWordsMessage(),
        );
        final WakeWordDetection detectionInfo = detectWakeWordResponse.unwrap();
        if (detectionInfo.detected) {
          // Transcribe audio data if wake word is detected
          _logger.i('Wake word detected');
          final transcribeResponse = await sendMessage(
            messageType: MessageType.transcribe,
            message: TranscribeMessage(),
          );
          final String transcribed = transcribeResponse.unwrap();

          // Remove detected wake word from transcript
          if (detectionInfo.startIdx != null && detectionInfo.endIdx != null) {
            transcript = transcribed.replaceRange(
              detectionInfo.startIdx!,
              detectionInfo.endIdx!,
              '',
            );
          } else {
            transcript = transcribed;
          }
        }
      }
    });
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

  /// Pauses the listener.
  Future<void> pauseListening() async {
    if (isListening) {
      isListening = false;
      await _recorder.stopRecorder();
    }
  }

  /// Closes the listener.
  Future<void> closeListener() async {
    if (isListening) {
      isListening = false;
      await _recorder.stopRecorder();
      await _recorder.closeRecorder();
    }
  }
}
