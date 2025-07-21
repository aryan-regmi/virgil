import 'dart:async';

import 'package:flutter/material.dart';
import 'package:virgil/speech_recognition.dart';

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
  State<HomePage> createState() => _HomePageState();
}

/// The state of the home page.
class _HomePageState extends State<HomePage> {
  final SpeechRecognition _speech = SpeechRecognition(
    wakeWords: ['Hey Virgil', 'Wake'],
  );

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      Timer.periodic(Duration(seconds: 1), (_) => setState(() {}));
    });
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final listenButton = ElevatedButton(
      onPressed: _speech.isListening
          ? null
          : () async => await _speech.startListening(),
      child: Text('Listen'),
    );
    final pauseButton = ElevatedButton(
      onPressed: _speech.isListening
          ? () async => await _speech.pauseListening()
          : null,
      child: Text('Pause'),
    );
    final stopButton = ElevatedButton(
      onPressed: _speech.isListening
          ? () async => await _speech.closeListener()
          : null,
      child: Text('Stop'),
    );
    final restartButton = ElevatedButton(
      onPressed: _speech.isListening
          ? null
          : () async {
              await _speech.closeListener();
              await _speech.restartListener();
            },
      child: Text('Restart'),
    );

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            listenButton,
            pauseButton,
            stopButton,
            restartButton,
            Text(_speech.transcript),
          ],
        ),
      ),
    );
  }
}
