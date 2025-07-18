import 'dart:async';

import 'package:flutter/material.dart';
import 'package:virgil/speech_recognition.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Demo',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
      ),
      home: const MyHomePage(title: 'Flutter Demo Home Page'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({super.key, required this.title});

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  final SpeechRecognition _speech = SpeechRecognition(wakeWords: ['Wake']);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      Timer.periodic(Duration(seconds: 0), (_) async {
        setState(() {});
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    var listenButton = ElevatedButton(
      onPressed: _speech.isListening
          ? null
          : () async => await _speech.startListening(),
      child: Text('Listen'),
    );
    var pauseButton = ElevatedButton(
      onPressed: _speech.isListening
          ? () async => await _speech.pauseListening()
          : null,
      child: Text('Pause'),
    );
    var stopButton = ElevatedButton(
      onPressed: _speech.isListening
          ? () async => await _speech.closeListener()
          : null,
      child: Text('Stop'),
    );

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[listenButton, pauseButton, stopButton],
        ),
      ),
    );
  }
}
