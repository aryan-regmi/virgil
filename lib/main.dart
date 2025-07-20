import 'dart:async';

import 'package:flutter/material.dart';
import 'package:virgil/native.dart';
import 'package:virgil/rust_bridge.dart';

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
  String? _text;

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
    final sendButton = ElevatedButton(
      onPressed: () async {
        final response = await sendMessage(
          messageType: MessageType.debug,
          message: DebugMessage(message: 'Hi from Flutter!'),
        );
        if (response.kind == ResponseType.text ||
            response.kind == ResponseType.error) {
          setState(() {
            _text = response.value;
          });
        }
      },
      child: Text('Send to Rust'),
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
            sendButton,
            _text == null ? Text('Waiting...') : Text(_text!),
          ],
        ),
      ),
    );
  }
}
