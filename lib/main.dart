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
  final LogLevel _level = LogLevel.debug;
  Context? _ctx;
  String transcript = '';

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
    // final listenBtn = ElevatedButton(
    //   onPressed: () async {
    //     final listenDurationMs = 1000;
    //     if (_ctx != null) {
    //       final textStream = transcribe(_ctx!, listenDurationMs);
    //       while (true) {
    //         await for (final text in textStream) {
    //           setState(() {
    //             transcript = text;
    //           });
    //           await Future.delayed(Duration(milliseconds: listenDurationMs));
    //         }
    //       }
    //     }
    //   },
    //   child: Text('Listen'),
    // );

    final listenBtn = ElevatedButton(
      onPressed: () async {
        if (_ctx != null) {}
        await transcribeMicInput(_ctx!, 1000);
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
            transcript.isNotEmpty
                ? Text('Transcript: $transcript')
                : Text('Waiting...'),
          ],
        ),
      ),
    );
  }
}
