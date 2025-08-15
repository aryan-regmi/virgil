import 'dart:async';
import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:logger/logger.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

/// The logger used for the application.
final logger = Logger(level: Level.debug);

void main() {
  runApp(const Virgil());
}

class Virgil extends StatelessWidget {
  const Virgil({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Virgil Assistant',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
      ),
      home: const HomePage(title: 'Virgil AI'),
    );
  }
}

/// The home page of the application.
class HomePage extends StatefulWidget {
  const HomePage({super.key, required this.title});

  final String title;

  @override
  State<HomePage> createState() => HomePageState();
}

/// The state of the home page.
class HomePageState extends State<HomePage> {
  /// The log level of the native library.
  final LogLevel _level = LogLevel.info;

  /// The context passed to the native library.
  Context? _ctx;

  /// The transcript.
  String _transcript = _defaultTranscript;

  /// Default transcript message.
  static const _defaultTranscript = 'Waiting...';

  /// The port used for FFI communications.
  final _receivePort = ReceivePort();

  /// Determines if the mic is listening.
  bool _listening = false;

  @override
  void initState() {
    super.initState();
    setupLogs(_level.index);

    // Download Whisper model and initalize Rust context
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await initFFI(_receivePort.sendPort.nativePort);

      final modelManager = await ModelManager.init();
      if (modelManager.modelPath != null) {
        _ctx = await initalizeContext(
          modelPath: modelManager.modelPath!,
          wakeWords: ['Wake', 'Test'],
        );
      } else {
        throw Exception('Failed to initalize Whisper model');
      }
    });
  }

  @override
  void dispose() {
    super.dispose();
    _receivePort.close();
    logger.close();
  }

  @override
  Widget build(BuildContext context) {
    final listenBtn = ElevatedButton(
      onPressed: () async {
        if (_listening) {
          setState(() {
            _listening = false;
          });
          stopMic();
        } else {
          if (_ctx != null) {
            setState(() {
              _listening = true;
            });
            await transcribeMicInput(_ctx!, 1000);
          }
        }
      },
      child: _listening ? Text('Stop') : Text('Listen'),
    );
    final streamBuilder = StreamBuilder(
      stream: _receivePort,
      builder: (ctx, snapshot) {
        if (_listening) {
          if (snapshot.hasData) {
            String? message = snapshot.data;
            if (message == null) {
              logger.e('Invalid message...');
              return Text('Invalid message');
            }
            _transcript = message;
            return Text(_transcript);
          }
        }
        return Text(_defaultTranscript);
      },
    );

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[listenBtn, streamBuilder],
        ),
      ),
    );
  }
}
