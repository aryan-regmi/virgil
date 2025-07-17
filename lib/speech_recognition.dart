import 'dart:async';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart' as ffi;
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_sound/flutter_sound.dart';
import 'package:http/http.dart' as http;
import 'package:logger/logger.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';

/// The logger responsible for all log messages from `SpeechRecognition`.
var _logger = Logger();

/// The native library to load.
final nativeLib = DynamicLibrary.open('libnative.so');

// Function types for native library.
typedef _LoadModelNativeFn = Void Function(Pointer<ffi.Utf8>);
typedef _LoadModelFn = void Function(Pointer<ffi.Utf8>);

/// Copy file from assets into application's documents directory.
Future<File> getFileFromAssets(String assetPath, String filename) async {
  final Directory docDir = await getApplicationDocumentsDirectory();
  final String localPath = docDir.path;
  final String targetFilePath = '$localPath/$filename';
  File targetFile = File(targetFilePath);
  final asset = await rootBundle.load(assetPath);
  final buffer = asset.buffer;
  return targetFile.writeAsBytes(
    buffer.asUint8List(asset.offsetInBytes, asset.lengthInBytes),
  );
}

/// Responsible for all speech recognition.
class SpeechRecognition {
  SpeechRecognition._init({required int sampleRate, required int numChannels})
    : _sampleRate = sampleRate,
      _numChannels = numChannels;

  /// The path to the Whisper model.
  String? modelPath;

  /// Determines if the microphone is listening.
  bool isListening = false;

  /// The sample rate in `Hz`.
  final int _sampleRate;

  /// The number of audio channels.
  final int _numChannels;

  /// The controller for the audio stream.
  final StreamController<List<Float32List>> _micStreamController =
      StreamController();

  /// Used to listen to the microphone.
  final FlutterSoundRecorder _listener = FlutterSoundRecorder();

  /// Initalizes the speech recognition module.
  static Future<SpeechRecognition> init({
    int sampleRate = 16_000,
    int numChannels = 1,
  }) async {
    var speech = SpeechRecognition._init(
      sampleRate: sampleRate,
      numChannels: numChannels,
    );

    // Download Whisper model
    var modelManager = await _WhisperModelManager.init();
    speech.modelPath = modelManager.modelPath;

    // Load and invoke the `load_model` function in Rust
    final loadModel = nativeLib
        .lookupFunction<_LoadModelNativeFn, _LoadModelFn>('load_model');
    loadModel(speech.modelPath!.toNativeUtf8());

    // Request permissions
    var status = await Permission.microphone.request();
    if (status != PermissionStatus.granted) {
      throw RecordingPermissionException('Microphone permissions not granted');
    }

    // Start listening once initalized
    speech._listener.openRecorder();

    return speech;
  }

  /// Starts speech recognition on microphone input.
  Future<void> startListening() async {
    // Record audio only if wake word is detected
    _micStreamController.stream.listen((channel) async {
      if (isListening) {
        // TODO: Handle stereo (more than one channel!)
        var channelAudio = channel[0];
        if (await _wakeWordDetected(channelAudio)) {
          var commands = await _transcribe(channelAudio);
          await _processCommands(commands);
        }
      }
    });

    // Start listening on the microphone
    await _listener.startRecorder(
      codec: Codec.pcmFloat32,
      sampleRate: _sampleRate,
      numChannels: _numChannels,
      audioSource: AudioSource.defaultSource,
      toStreamFloat32: _micStreamController.sink,
    );
    isListening = true;
  }

  /// Pauses the listener.
  Future<void> pauseListening() async {
    if (isListening) {
      isListening = false;
      await _listener.stopRecorder();
    }
  }

  /// Closes the listener.
  Future<void> closeListener() async {
    if (isListening) {
      isListening = false;
      await _listener.stopRecorder();
      await _listener.closeRecorder();
    }
  }

  /// Converts raw audio data to text by running speech recognition.
  Future<String> _transcribe(Float32List audioData) async {
    // TODO: Call rust code
    return 'TODO';
  }

  /// Processes the user's commands.
  Future<void> _processCommands(String text) async {
    _logger.i('Command: $text');
  }

  /// Checks if the wake word is detected.
  Future<bool> _wakeWordDetected(Float32List audioData) async {
    // TODO: Pass to recognition func in Rust
    return false;
  }
}

// TODO: Choose model based on device locale.
//
/// Responsible for downloading the `Whisper` model if necessary.
class _WhisperModelManager {
  _WhisperModelManager._init();

  /// The path of the downloaded model.
  String? modelPath;

  /// Name of the model.
  static const String _modelName = 'ggml-tiny.bin';

  /// Url to download the model from.
  static const String _modelUrl =
      'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$_modelName';

  /// Initalizes the Whisper model manager.
  static Future<_WhisperModelManager> init() async {
    var manager = _WhisperModelManager._init();
    manager.modelPath = await _WhisperModelManager._downloadModel();
    return manager;
  }

  // TODO: Add default model in `models/` so no network permission is required?
  //
  /// Downloads the model if necessary and returns its path.
  static Future<String> _downloadModel() async {
    final documentsDir = await getApplicationDocumentsDirectory();
    final modelPath = '${documentsDir.path}/$_modelName';
    final file = File(modelPath);

    if (await file.exists()) {
      _logger.i('Existing model found at $modelPath');
      return modelPath;
    }

    _logger.i('Downloading `$_modelName` Whisper model from $_modelUrl ...');
    final response = await http.get(Uri.parse(_modelUrl));
    if (response.statusCode == 200) {
      await file.writeAsBytes(response.bodyBytes);
      _logger.i('Model download to $modelPath');
      return modelPath;
    } else {
      throw Exception(
        'Failed to download Whisper model: ${response.statusCode}',
      );
    }
  }
}
