import 'dart:async';

sealed class AetherAppException implements Exception {
  final String message;
  final Object? cause;
  const AetherAppException(this.message, [this.cause]);
  @override
  String toString() => message;
}

final class AetherNetworkException extends AetherAppException {
  const AetherNetworkException(super.message, [super.cause]);
}

final class AetherValidationException extends AetherAppException {
  const AetherValidationException(super.message, [super.cause]);
}

final class AetherBackendException extends AetherAppException {
  const AetherBackendException(super.message, [super.cause]);
}

String userFacingError(Object error) {
  if (error is AetherAppException) return error.message;
  if (error is TimeoutException) return 'İşlem zaman aşımına uğradı. Lütfen tekrar dene.';
  if (error is FormatException) return 'Sunucudan beklenmeyen veri alındı.';
  final raw = error.toString().replaceFirst('Exception: ', '').trim();
  if (raw.isEmpty) return 'Beklenmeyen bir hata oluştu.';
  return raw.length > 240 ? '${raw.substring(0, 237)}...' : raw;
}
