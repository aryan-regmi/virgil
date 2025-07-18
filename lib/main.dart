import 'package:flutter/material.dart';
import 'package:virgil/messages.dart';
import 'package:virgil/model_manager.dart';
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
  late SpeechRecognition _speech;

  @override
  void initState() {
    super.initState();
    _speech = SpeechRecognition();
  }

  @override
  void dispose() {
    _speech.closeListener();
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

    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[listenButton, pauseButton],
        ),
      ),
    );
  }
}
