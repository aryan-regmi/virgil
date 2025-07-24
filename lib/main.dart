import 'dart:async';

import 'package:flutter/material.dart';
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
  bool _detected = false;

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
    // while (true) {
    //   listenToMic();
    //   await Future.delayed(Duration(seconds: 2));
    // }

    final initCtxBtn = ElevatedButton(
      onPressed: () async {
        _ctx = await initalizeContext(
          modelPath: 'native/test_assets/ggml-tiny.bin',
          wakeWords: ['Wake'],
        );
      },
      child: Text('Init Context'),
    );

    final detectWakeWordsBtn = ElevatedButton(
      onPressed: () async {
        final listenDurationMs = 1000;
        if (_ctx != null) {
          _detected = await detectWakeWords(_ctx!, listenDurationMs);
          await Future.delayed(Duration(milliseconds: listenDurationMs));
        }
      },
      child: Text('Detect Wake Word'),
    );

    final activeListenBtn = ElevatedButton(
      onPressed: () async {
        final listenDurationMs = 1000;
        if (_ctx != null) {
          _ctx = await activeListeningMode(_ctx!, listenDurationMs);
          await Future.delayed(Duration(milliseconds: listenDurationMs));
        }
      },
      child: Text('Active Listen'),
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
            initCtxBtn,
            detectWakeWordsBtn,
            activeListenBtn,
            _detected ? Text('Wake word detected!') : Text(''),
            _ctx != null
                ? Text('Transcript: ${_ctx!.transcript}')
                : Text('Waiting...'),
          ],
        ),
      ),
    );
  }
}
