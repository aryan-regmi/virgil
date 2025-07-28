import 'dart:async';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:flutter/material.dart';
import 'package:logger/web.dart';
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

// final _logger = Logger();
String transcript = '';
void rustCallback(Pointer<Utf8> textPtr) {
  transcript = textPtr.toDartString();
  _HomePageState._streamController.add(transcript);
}

/// The state of the home page.
class _HomePageState extends State<HomePage> {
  final LogLevel _level = LogLevel.info;
  Context? _ctx;
  String _transcript = '';
  static final _streamController = StreamController<String>.broadcast();

  @override
  void initState() {
    super.initState();
    setupLogs(_level.index);

    WidgetsBinding.instance.addPostFrameCallback((_) async {
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
          await transcribeInIsolate(_ctx!, 1000);
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
            StreamBuilder(
              stream: _streamController.stream,
              builder: (ctx, snapshot) {
                if (!snapshot.hasData) return Text('Waiting...');
                setState(() {
                  _transcript = snapshot.data!;
                });
                return Text(_transcript);
              },
            ),
          ],
        ),
      ),
    );
  }
}
