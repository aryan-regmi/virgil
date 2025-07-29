import 'dart:async';
import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:logger/logger.dart';
import 'package:virgil/model_manager.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

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

final logger = Logger(level: Level.debug);

/// The state of the home page.
class HomePageState extends State<HomePage> {
  final LogLevel _level = LogLevel.info;
  Context? _ctx;
  String _transcript = 'Waiting...';
  late StreamSubscription<dynamic> _portListener;

  @override
  void initState() {
    super.initState();
    setupLogs(_level.index);

    /// The port used for FFI communications.
    final receivePort = ReceivePort();

    // Download Whisper model and initalize Rust context
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await initFFI(receivePort.sendPort.nativePort);

      // Setup Port listener
      _portListener = receivePort.listen((message) {
        logger.i(message);
        if (message == null) {
          logger.e("Invalid message...");
        }
        setState(() {
          _transcript = message;
        });
      });

      final modelManager = await ModelManager.init();
      if (modelManager.modelPath != null) {
        _ctx = await initalizeContext(
          modelPath: modelManager.modelPath!,
          wakeWords: ['Wake', 'Test'],
        );
      } else {
        throw Exception('Failed to initalize Whispe model');
      }
    });
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final listenBtn = ElevatedButton(
      onPressed: () async {
        if (_ctx != null) {
          await transcribeMicInput(_ctx!, 1000);
        }
      },
      child: Text('Listen'),
    );

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[listenBtn, Text(_transcript)],
        ),
      ),
    );
  }
}
