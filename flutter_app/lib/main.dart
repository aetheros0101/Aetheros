import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api/aetheros.dart' as aether;
import 'screens/home_screen.dart';
import 'services/ai_provider_service.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  String? initError;

  try {
    // 1. FRB bridge başlat
    await RustLib.init();

    // 2. DB yolu
    final docDir = await getApplicationDocumentsDirectory();
    final dbPath = '${docDir.path}/aetheros.db';

    // 3. Rust runtime başlat
    await aether.initializeRuntime(
      dbPath: dbPath,
      workerCount: 2,
    );

    // 4. Ayarlar'da kayıtlı AI provider'ları Rust router'a yeniden yükle.
    //    ProviderRouter in-memory'dir — her runtime başlangıcında boştur.
    await AiProviderService.rehydrateFromStorage();
  } catch (e, stack) {
    // Hata yakalandı — siyah ekran yerine hata göster
    initError = '$e\n\n$stack';
    debugPrint('AetherOS init error: $e\n$stack');
  }

  runApp(
    ProviderScope(
      child: initError != null
          ? _ErrorApp(error: initError!)
          : const AetherOSApp(),
    ),
  );
}

/// Normal uygulama
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
      ),
      home: const HomeScreen(),
    );
  }
}

/// Başlatma hatası ekranı — siyah ekran yerine hatayı gösterir
class _ErrorApp extends StatelessWidget {
  final String error;
  const _ErrorApp({required this.error});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(),
      home: Scaffold(
        backgroundColor: const Color(0xFF1a1a2e),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Row(children: [
                  Icon(Icons.error_outline, color: Colors.red, size: 28),
                  SizedBox(width: 8),
                  Text('AetherOS Başlatma Hatası',
                      style: TextStyle(
                          color: Colors.red,
                          fontSize: 18,
                          fontWeight: FontWeight.bold)),
                ]),
                const SizedBox(height: 16),
                Expanded(
                  child: Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.black45,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: SingleChildScrollView(
                      child: SelectableText(
                        error,
                        style: const TextStyle(
                            color: Colors.white70,
                            fontSize: 11,
                            fontFamily: 'monospace'),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
