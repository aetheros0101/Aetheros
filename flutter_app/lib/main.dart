import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api/aetheros.dart' as aether;
import 'services/ai_provider_service.dart';
import 'ui/design/aether_theme.dart';
import 'application/application.dart';
import 'ui/shell/aether_workbench.dart';
import 'screens/home_screen.dart';
import 'ui/shell/accessibility_scope.dart';

/// Feature flag: when true, open Workbench shell; otherwise legacy HomeScreen.
/// Toggle during migration without removing runtime init or old screens.
const bool kUseWorkbench = true;

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  String? initError;
  AetherApplication? app;

  try {
    // 1. FRB bridge — DO NOT REMOVE
    await RustLib.init();

    // 2. DB path
    final docDir = await getApplicationDocumentsDirectory();
    final dbPath = '${docDir.path}/aetheros.db';

    // 3. Rust runtime — DO NOT REMOVE
    // Android süreci activity kapansa da canlı kalabilir; Rust statik durumu
    // da öyle. Dart tarafı yeniden başlarken runtime zaten hazırsa tekrar
    // başlatma ("Runtime zaten başlatılmış" hatasını önler).
    if (!await aether.isRuntimeReady()) {
      await aether.initializeRuntime(
        dbPath: dbPath,
        workerCount: 2,
      );
    }

    // 4. Rehydrate AI providers into Rust router — DO NOT REMOVE
    await AiProviderService.rehydrateFromStorage();

    // 5. Application composition (Workbench state/commands/services)
    app = AetherApplication();
    await app.initializeWorkspace();
    await app.restoreLayout();
  } catch (e, stack) {
    initError = '$e\n\n$stack';
    debugPrint('AetherOS init error: $e\n$stack');
  }

  runApp(
    ProviderScope(
      child: initError != null
          ? _ErrorApp(error: initError!)
          : AetherOSApp(application: app),
    ),
  );
}

class AetherOSApp extends StatelessWidget {
  const AetherOSApp({super.key, this.application});

  final AetherApplication? application;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'AetherOS',
      debugShowCheckedModeBanner: false,
      theme: buildAetherTheme(),
      locale: const Locale('tr'),
      supportedLocales: const [Locale('tr'), Locale('en')],
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      builder: (context, child) => AccessibilityHost(child: child ?? const SizedBox.shrink()),
      home: kUseWorkbench && application != null
          ? AetherWorkbench(app: application!)
          : const HomeScreen(),
      routes: {
        '/legacy': (_) => const HomeScreen(),
      },
    );
  }
}

class _ErrorApp extends StatelessWidget {
  final String error;
  const _ErrorApp({required this.error});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(),
      home: Scaffold(
        backgroundColor: Colors.black,
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: SingleChildScrollView(
              child: SelectableText(
                'AetherOS başlatılamadı:\n\n$error',
                style: const TextStyle(color: Colors.redAccent, fontSize: 13),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
