// ============================================================
// flutter_app/lib/src/rust/frb_generated.dart
//
// ⚠️  BU DOSYA OTOMATİK ÜRETİLİR — EL İLE DÜZENLEMEYİN
//
// Üretmek için:
//   cd flutter_app
//   dart run flutter_rust_bridge_codegen generate
//
// Bu dosya şu an sadece derleme için bir placeholder.
// Gerçek dosya codegen sonrası oluşur ve dart pub get ile
// projeye dahil edilir.
// ============================================================

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

// ignore_for_file: unused_import, prefer_const_constructors

class RustLib extends BaseEntrypoint<RustLibApi, RustLibApiImpl, RustLibWire> {
  @internal
  static final instance = RustLib._();

  RustLib._();

  static Future<void> init({
    RustLibApi? api,
    BaseHandler? handler,
    ExternalLibrary? externalLibrary,
  }) async {
    await instance.initImpl(
      api: api,
      handler: handler,
      externalLibrary: externalLibrary,
    );
  }

  @override
  ApiImplConstructor<RustLibApiImpl, RustLibWire> get apiImplConstructor =>
      RustLibApiImpl.new;

  @override
  WireConstructor<RustLibWire> get wireConstructor =>
      RustLibWire.fromExternalLibrary;

  @override
  Future<void> executeRustInitializers() async {
    // Rust init_app() çağrısı codegen sonrası buraya eklenir
  }

  @override
  ExternalLibraryLoaderConfig get defaultExternalLibraryLoaderConfig =>
      kDefaultExternalLibraryLoaderConfig;

  @override
  String get codegenVersion => '2.9.0';

  @override
  int get rustContentHash => 0; // codegen günceller
}

// Bu sınıflar codegen tarafından doldurulur:
abstract class RustLibApi extends BaseApi {}
class RustLibApiImpl extends RustLibApiImplPlatform implements RustLibApi {
  RustLibApiImpl({required super.handler});
}
class RustLibWire extends BaseWire {
  RustLibWire.fromExternalLibrary(super.lib);
}

const kDefaultExternalLibraryLoaderConfig = ExternalLibraryLoaderConfig(
  stem: 'aetheros',
  ioDirectory: 'rust/target/release/',
  webPrefix: 'pkg/',
);
