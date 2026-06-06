// ============================================================
// flutter_app/lib/main.dart
//
// AetherOS mobil uygulaması giriş noktası.
//
// BAŞLANGIÇ SIRASI:
//   1. FRB altyapısı init et  (RustLib.init)
//   2. AetherOS runtime başlat (Rust tarafı)
//   3. Flutter UI başlat
//
// Tüm Rust çağrıları `aetheros/` Dart API'si üzerinden yapılır.
// Dart kodu hiçbir zaman native FFI'ya doğrudan erişmez.
// ============================================================

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api/aetheros.dart' as aether;
import 'screens/home_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 1. FRB platform kanallarını başlat
  await RustLib.init();

  // 2. Veritabanı yolunu belirle
  final docDir = await getApplicationDocumentsDirectory();
  final dbPath = '${docDir.path}/aetheros.db';

  // 3. AetherOS runtime başlat
  //    worker_count: cihazın çekirdek sayısına göre (2-4 önerilir)
  await aether.initializeRuntime(
    dbPath: dbPath,
    workerCount: 2,
  );

  runApp(
    const ProviderScope(
      child: AetherOSApp(),
    ),
  );
}

class AetherOSApp extends StatelessWidget {
  const AetherOSApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'AetherOS',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF6C63FF),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
        fontFamily: 'Roboto',
      ),
      home: const HomeScreen(),
    );
  }
}
