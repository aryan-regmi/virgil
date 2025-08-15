import 'dart:async';

import 'package:flutter/material.dart';
import 'package:logger/logger.dart';
import 'package:virgil/native.dart';
import 'package:virgil/speech_recognition.dart';

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
  SpeechRecognition speech = SpeechRecognition(LogLevel.info);
  bool _isListening = false;
  late Timer _timer;
  String _transcript = '';

  @override
  void initState() {
    super.initState();

    // Download Whisper model and initalize Rust context
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      speech.init();
    });
  }

  @override
  void dispose() {
    super.dispose();
    speech.dispose();
    logger.close();
  }

  @override
  Widget build(BuildContext context) {
    final listenBtn = ElevatedButton(
      onPressed: () async {
        if (speech.isListening) {
          await speech.stopListening();
          setState(() {
            _isListening = false;
          });
          _timer.cancel();
        } else {
          _timer = Timer.periodic(Duration(milliseconds: 1000), (_) async {
            await speech.processCommands();
            setState(() {
              _transcript = speech.command == null ? '' : speech.command!;
            });
          });

          await speech.startListening();
          setState(() {
            _isListening = true;
          });
        }
      },
      child: _isListening ? Text('Stop') : Text('Listen'),
    );
    // final streamBuilder = speech.streamBuilder();

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          // children: <Widget>[listenBtn, streamBuilder],
          children: <Widget>[listenBtn, Text(_transcript)],
        ),
      ),
    );
  }
}
