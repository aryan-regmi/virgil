import 'package:flutter/material.dart';
import 'package:virgil/src/rust/api/simple.dart';
import 'package:virgil/src/rust/frb_generated.dart';
import 'package:virgil/src/speech_recognition.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const Virgil());
}

/// The root of the appliation.
class Virgil extends StatelessWidget {
  const Virgil({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Virgil',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
      ),
      home: const HomePage(title: 'Virgil AI Assistant'),
    );
  }
}

/// The homepage of the app.
class HomePage extends StatefulWidget {
  const HomePage({super.key, required this.title});

  /// Homepage title.
  final String title;

  @override
  State<HomePage> createState() => _HomePageState();
}

/// The state of the home page.
class _HomePageState extends State<HomePage> {
  late SpeechRecognition _speech;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      _speech = await SpeechRecognition.init();
      getModelPath(name: _speech.modelPath!); // Send path to rust
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
          children: [listenButton, pauseButton, stopButton],
        ),
      ),
    );
  }
}
