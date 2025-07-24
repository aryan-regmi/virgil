/// Contains model manager for downloading and storing `Whisper` models.
library;

import 'dart:io';

import 'package:logger/web.dart';
import 'package:path_provider/path_provider.dart';
import 'package:http/http.dart' as http;

final _logger = Logger();

// TODO: Change models based on locale?

/// Manages the `Whisper` model used.
class ModelManager {
  ModelManager._init();

  /// The path of the downloaded model.
  String? modelPath;

  /// Name of the model.
  static const String _modelName = 'ggml-tiny.bin';

  /// Url to download the model from.
  static const String _modelUrl =
      'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$_modelName';

  /// Initalizes the Whisper model manager and downlods the model.
  static Future<ModelManager> init() async {
    var manager = ModelManager._init();
    manager.modelPath = await ModelManager._downloadModel();
    return manager;
  }

  // TODO: Add default model in `models/` so no network permission is required?
  //
  /// Downloads the model if necessary and returns its path.
  static Future<String> _downloadModel() async {
    final documentsDir = await getApplicationDocumentsDirectory();
    final modelPath = '${documentsDir.path}/$_modelName';
    final file = File(modelPath);

    if (await file.exists()) {
      _logger.i('Existing model found at $modelPath');
      return modelPath;
    }

    _logger.i('Downloading `$_modelName` Whisper model from $_modelUrl ...');
    final response = await http.get(Uri.parse(_modelUrl));
    if (response.statusCode == 200) {
      await file.writeAsBytes(response.bodyBytes);
      _logger.i('Model downloaded to $modelPath');
      return modelPath;
    } else {
      throw Exception(
        'Failed to download Whisper model: ${response.statusCode}',
      );
    }
  }
}
