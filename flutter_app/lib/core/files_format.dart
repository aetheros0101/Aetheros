// ============================================================
// flutter_app/lib/core/files_format.dart
//
// Dosyalar ekranının saf (Flutter'dan bağımsız) yardımcıları — `flutter test`
// ile test edilir. Yollar workspace köküne görelidir ve '/' ile ayrılır;
// kök "" ile gösterilir.
// ============================================================

/// "0 B", "1.5 KB", "2.0 MB".
String formatSize(int bytes) {
  if (bytes < 0) return '?';
  if (bytes < 1024) return '$bytes B';
  final kb = bytes / 1024;
  if (kb < 1024) return '${kb.toStringAsFixed(1)} KB';
  final mb = kb / 1024;
  if (mb < 1024) return '${mb.toStringAsFixed(1)} MB';
  return '${(mb / 1024).toStringAsFixed(1)} GB';
}

/// "a/b/c.txt" → "a/b"; "a" → ""; "" → "".
String parentOf(String path) {
  final i = path.lastIndexOf('/');
  return i < 0 ? '' : path.substring(0, i);
}

/// `dir` + `name` → "dir/name" (`dir` boşsa yalnız ad).
String joinPath(String dir, String name) {
  final d = dir.replaceAll(RegExp(r'^/+|/+$'), '');
  return d.isEmpty || d == '.' ? name : '$d/$name';
}

/// Yolun son bileşeni.
String baseName(String path) {
  final i = path.lastIndexOf('/');
  return i < 0 ? path : path.substring(i + 1);
}

class Crumb {
  final String label;
  final String path;
  const Crumb(this.label, this.path);
}

/// "a/b" → [Çalışma alanı (""), a ("a"), b ("a/b")].
List<Crumb> breadcrumbs(String path, {String rootLabel = 'Çalışma alanı'}) {
  final out = <Crumb>[Crumb(rootLabel, '')];
  var acc = '';
  for (final part in path.split('/')) {
    if (part.isEmpty || part == '.') continue;
    acc = acc.isEmpty ? part : '$acc/$part';
    out.add(Crumb(part, acc));
  }
  return out;
}

/// Yeni ad için hata metni; geçerliyse null. Rust ile aynı kurallar
/// (`bridge::files::check_name`): boş, '.', '..', '/', '\', NUL yok.
String? validateEntryName(String name) {
  final n = name.trim();
  if (n.isEmpty || n == '.' || n == '..') return 'Geçersiz ad.';
  if (n.contains('/') || n.contains('\\') || n.contains('\u0000')) {
    return "Ad '/' veya '\\' içeremez.";
  }
  return null;
}

/// `existing` içinde olmayan ilk adı döner: "a.txt" → "a (1).txt" → "a (2).txt".
/// Nokta ile başlayan adlarda ("`.env`") tamamı ad sayılır: ".env (1)".
String uniqueName(Iterable<String> existing, String name) {
  final taken = existing.toSet();
  if (!taken.contains(name)) return name;
  final dot = name.lastIndexOf('.');
  final hasExt = dot > 0 && dot < name.length - 1;
  final stem = hasExt ? name.substring(0, dot) : name;
  final ext = hasExt ? name.substring(dot) : '';
  var i = 1;
  while (taken.contains('$stem ($i)$ext')) {
    i++;
  }
  return '$stem ($i)$ext';
}

/// Rust, dosya sen açtıktan sonra değiştiyse "conflict:" önekli hata döner.
bool isWriteConflict(Object error) => error.toString().contains('conflict:');

/// Hata metninden "conflict:" önekini atar (kullanıcıya gösterilecek metin).
String stripConflictPrefix(String message) =>
    message.replaceFirst(RegExp(r'^.*?conflict:\s*'), '');

/// Dosya adına göre simge sınıfı (arayüz ikon seçsin diye).
enum FileKind { folder, text, code, data, image, other }

FileKind kindOf(String name, {required bool isDir}) {
  if (isDir) return FileKind.folder;
  final dot = name.lastIndexOf('.');
  final ext = dot < 0 ? '' : name.substring(dot + 1).toLowerCase();
  const code = {'rs', 'dart', 'py', 'js', 'ts', 'sh', 'toml', 'yaml', 'yml', 'wat', 'c', 'h', 'java', 'kt'};
  const data = {'json', 'csv', 'xml', 'log'};
  const image = {'png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'};
  const text = {'txt', 'md', 'rst'};
  if (code.contains(ext)) return FileKind.code;
  if (data.contains(ext)) return FileKind.data;
  if (image.contains(ext)) return FileKind.image;
  if (text.contains(ext) || ext.isEmpty) return FileKind.text;
  return FileKind.other;
}
