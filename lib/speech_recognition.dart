// import 'dart:async';
// import 'dart:convert';
// import 'dart:ffi';
//
// import 'package:ffi/ffi.dart';
// import 'package:flutter/foundation.dart';
// import 'package:flutter_sound/flutter_sound.dart';
// import 'package:logger/web.dart';
// import 'package:permission_handler/permission_handler.dart';
// import 'package:virgil/model_manager.dart';
//
// final _logger = Logger();
//
// /// Runs speech recognition on microphone input and processes the correrponding commands.
// class SpeechRecognition {
//   SpeechRecognition({int sampleRate = 44_100, int numChannels = 1})
//     : _sampleRate = sampleRate,
//       _numChannels = numChannels {
//     _init();
//   }
//
//   /// Determines if the microphone is listening.
//   bool isListening = false;
//
//   String? text;
//
//   /// The sample rate in `Hz`.
//   final int _sampleRate;
//
//   /// The number of audio channels.
//   final int _numChannels;
//
//   /// The controller for the audio stream.
//   final StreamController<List<Float32List>> _micStreamController =
//       StreamController();
//
//   /// Used to listen to the microphone.
//   final FlutterSoundRecorder _listener = FlutterSoundRecorder();
//
//   /// Initalizes the speech recognition module.
//   Future<void> _init() async {
//     // Download Whisper model and load it
//     final modelManager = await ModelManager.init();
//     final modelPath = modelManager.modelPath;
//     if (modelPath != null) {
//       await sendModelPathAndLoadModel(modelPath);
//     } else {
//       throw Exception('Invalid model path');
//     }
//
//     // Request permissions
//     var status = await Permission.microphone.request();
//     if (status != PermissionStatus.granted) {
//       throw RecordingPermissionException('Microphone permissions not granted');
//     }
//
//     // Initalize microphone
//     _listener.openRecorder();
//
//     // Initalize listener that runs speech recognition
//     _micStreamController.stream.listen((channel) async {
//       if (isListening) {
//         // TODO: Handle stereo (more than one channel!)
//         var channelAudio = channel[0];
//         await sendAudioData(channelAudio);
//
//         // Process commands only if wake word is detected
//         if (wakeWordDetected()) {
//           _logger.i('Wake word detected');
//           var commands = await _transcribe();
//           await _processCommands(commands);
//         }
//       }
//     });
//   }
//
//   /// Starts listening to the microphone.
//   Future<void> startListening() async {
//     await _listener.startRecorder(
//       codec: Codec.pcmFloat32,
//       sampleRate: _sampleRate,
//       numChannels: _numChannels,
//       audioSource: AudioSource.defaultSource,
//       toStreamFloat32: _micStreamController.sink,
//     );
//     isListening = true;
//   }
//
//   /// Pauses the listener.
//   Future<void> pauseListening() async {
//     if (isListening) {
//       isListening = false;
//       await _listener.stopRecorder();
//     }
//   }
//
//   /// Closes the listener.
//   Future<void> closeListener() async {
//     if (isListening) {
//       isListening = false;
//       await _listener.stopRecorder();
//       await _listener.closeRecorder();
//     }
//   }
//
//   /// Converts raw microphone audio to text.
//   Future<String> _transcribe() async {
//     final lenPtr = calloc<Uint64>();
//     final dataPtr = transcribe(lenPtr);
//     final length = lenPtr.value;
//     calloc.free(lenPtr);
//
//     final bytes = dataPtr.asTypedList(length);
//     final text = utf8.decode(bytes);
//     freeTranscript(dataPtr, length);
//
//     return text;
//   }
//
//   // TODO: Move to another class/outside of this?
//   //
//   /// Processes the transcribe commands.
//   Future<void> _processCommands(String commands) async {
//     _logger.i('Command received: $commands');
//     text = commands;
//     // FIXME: Finish impl!
//   }
// }
