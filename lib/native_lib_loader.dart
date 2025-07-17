import 'dart:ffi';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:path_provider/path_provider.dart';

class NativeLibLoader {
  NativeLibLoader(String libName) : _libName = libName {
    var arch = Abi.current();
    switch (arch) {
      case Abi.androidX64:
        _arch = 'x86_64-linux-android';
        break;
      case Abi.androidArm64:
        _arch = 'aarch64-linux-android';
        break;
      default:
        _arch = 'armv7-linux-androideabi';
    }
  }

  String? _arch;
  final String _libName;

  DynamicLibrary? nativeLib;

  /// Load the library.
  Future<void> loadLib() async {
    final libFile = await getFileFromAssets(
      'native/target/$_arch/release/$_libName',
      _libName,
    );
    await getFileFromAssets(
      'native/target/$_arch/release/libc++_shared.so',
      'libc++_shared.so',
    );
    nativeLib = DynamicLibrary.open(libFile.path);
  }

  /// Copy file from assets into application's documents directory.
  Future<File> getFileFromAssets(String assetPath, String filename) async {
    final Directory docDir = await getApplicationDocumentsDirectory();
    final String localPath = docDir.path;
    final String targetFilePath = '$localPath/$filename';
    File targetFile = File(targetFilePath);
    final asset = await rootBundle.load(assetPath);
    final buffer = asset.buffer;
    return targetFile.writeAsBytes(
      buffer.asUint8List(asset.offsetInBytes, asset.lengthInBytes),
    );
  }
}
