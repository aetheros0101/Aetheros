import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/files_format.dart';

void main() {
  test('formatSize', () {
    expect(formatSize(0), '0 B');
    expect(formatSize(1023), '1023 B');
    expect(formatSize(1536), '1.5 KB');
    expect(formatSize(2 * 1024 * 1024), '2.0 MB');
    expect(formatSize(-1), '?');
  });

  test('parentOf / joinPath / baseName', () {
    expect(parentOf('a/b/c.txt'), 'a/b');
    expect(parentOf('a'), '');
    expect(parentOf(''), '');
    expect(joinPath('', 'x.txt'), 'x.txt');
    expect(joinPath('.', 'x.txt'), 'x.txt');
    expect(joinPath('d', 'x.txt'), 'd/x.txt');
    expect(joinPath('/d/e/', 'x.txt'), 'd/e/x.txt');
    expect(baseName('a/b/c.txt'), 'c.txt');
    expect(baseName('c.txt'), 'c.txt');
  });

  test('breadcrumbs kökten başlar ve kümülatif yol üretir', () {
    final root = breadcrumbs('');
    expect(root.length, 1);
    expect(root.single.path, '');
    final c = breadcrumbs('a/b');
    expect(c.map((e) => e.label).toList(), ['Çalışma alanı', 'a', 'b']);
    expect(c.map((e) => e.path).toList(), ['', 'a', 'a/b']);
    expect(breadcrumbs('a//b/.').map((e) => e.path).toList(), ['', 'a', 'a/b']);
  });

  test('validateEntryName Rust kurallarıyla aynı', () {
    expect(validateEntryName('notlar.txt'), isNull);
    expect(validateEntryName('  a b  '), isNull);
    for (final bad in ['', '  ', '.', '..', 'a/b', r'a\b', 'a\u0000b']) {
      expect(validateEntryName(bad), isNotNull, reason: bad);
    }
  });

  test('uniqueName çakışmada sayaç ekler, uzantıyı korur', () {
    expect(uniqueName(['b.txt'], 'a.txt'), 'a.txt');
    expect(uniqueName(['a.txt'], 'a.txt'), 'a (1).txt');
    expect(uniqueName(['a.txt', 'a (1).txt'], 'a.txt'), 'a (2).txt');
    expect(uniqueName(['arsiv.tar.gz'], 'arsiv.tar.gz'), 'arsiv.tar (1).gz');
    expect(uniqueName(['.env'], '.env'), '.env (1)');
    expect(uniqueName(['README'], 'README'), 'README (1)');
  });

  test('çakışma hatası tanınır ve öneki atılır', () {
    const msg = 'conflict: Dosya sen açtıktan sonra değişti.';
    expect(isWriteConflict(msg), isTrue);
    expect(isWriteConflict(Exception(msg)), isTrue);
    expect(isWriteConflict('başka hata'), isFalse);
    expect(stripConflictPrefix(msg), 'Dosya sen açtıktan sonra değişti.');
    expect(stripConflictPrefix('Exception: conflict: x'), 'x');
  });

  test('kindOf', () {
    expect(kindOf('x', isDir: true), FileKind.folder);
    expect(kindOf('a.rs', isDir: false), FileKind.code);
    expect(kindOf('a.JSON', isDir: false), FileKind.data);
    expect(kindOf('a.png', isDir: false), FileKind.image);
    expect(kindOf('notlar.txt', isDir: false), FileKind.text);
    expect(kindOf('README', isDir: false), FileKind.text);
    expect(kindOf('a.bin', isDir: false), FileKind.other);
  });
}
