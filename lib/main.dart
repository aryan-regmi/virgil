import 'dart:async';

import 'package:flutter/material.dart';
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
  State<HomePage> createState() => _HomePageState();
}

/// The state of the home page.
class _HomePageState extends State<HomePage> {
  Context? _ctx;

  @override
  void initState() {
    super.initState();

    WidgetsBinding.instance.addPostFrameCallback((_) async {
      // Timer.periodic(Duration(seconds: 1), (_) => setState(() {}));

      final modelManager = await ModelManager.init();
      if (modelManager.modelPath != null) {
        _ctx = Context(
          modelPath: modelManager.modelPath!,
          wakeWords: ['Wake', 'Test'],
          transcript: '',
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
    // while (true) {
    //   listenToMic();
    //   await Future.delayed(Duration(seconds: 2));
    // }

    final listenBtn = ElevatedButton(
      onPressed: () async {
        while (true) {
          final listenDurationMs = 1000;
          if (_ctx != null) {
            final text = await transcribe(_ctx!, listenDurationMs);
            setState(() {
              _ctx!.transcript = text;
            });
            await Future.delayed(Duration(milliseconds: listenDurationMs));
          }
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
          children: <Widget>[
            listenBtn,
            _ctx != null
                ? Text('Transcript: ${_ctx!.transcript}')
                : Text('Waiting...'),
          ],
        ),
      ),
    );
  }
}
