import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/approval_format.dart';

void main() {
  group('quoteArg', () {
    test('safe args stay as-is', () {
      expect(quoteArg('status'), 'status');
      expect(quoteArg('--stat'), '--stat');
      expect(quoteArg('HEAD~1'), 'HEAD~1');
      expect(quoteArg('src/main.rs'), 'src/main.rs');
    });

    test('empty and spaced args are quoted', () {
      expect(quoteArg(''), "''");
      expect(quoteArg('a b'), "'a b'");
    });

    test('shell metacharacters are quoted, single quotes escaped', () {
      expect(quoteArg('a && b'), "'a && b'");
      expect(quoteArg(r'$(id)'), r"'$(id)'");
      expect(quoteArg("it's"), r"'it'\''s'");
    });
  });

  group('commandLine', () {
    test('terminal shows just the command', () {
      expect(commandLine('terminal', ['git', 'commit', '-m', 'fix bug']),
          "git commit -m 'fix bug'");
    });

    test('terminal with no args is flagged', () {
      expect(commandLine('terminal', []), '(boş komut)');
    });

    test('other tools are prefixed with their name', () {
      expect(commandLine('wasm_runner', ['run', 'x y']), "wasm_runner run 'x y'");
      expect(commandLine('wasm_runner', []), 'wasm_runner');
    });
  });

  group('ageLabel', () {
    const now = 10 * 60 * 60 * 1000; // keyfi sabit "şimdi"
    test('buckets', () {
      expect(ageLabel(now - 10 * 1000, now), 'az önce');
      expect(ageLabel(now - 5 * 60 * 1000, now), '5 dk önce');
      expect(ageLabel(now - 2 * 60 * 60 * 1000, now), '2 sa önce');
      expect(ageLabel(0, 3 * 24 * 60 * 60 * 1000), '3 gün önce');
    });

    test('clock skew (future createdAt) does not go negative', () {
      expect(ageLabel(now + 5000, now), 'az önce');
    });
  });

  group('remainingLabel', () {
    const now = 100 * 60 * 1000;
    test('shows approximate minutes left', () {
      expect(remainingLabel(now - 10 * 60 * 1000, now), '~50 dk kaldı');
    });

    test('under a minute', () {
      expect(remainingLabel(now - (60 * 60 * 1000 - 30 * 1000), now), '<1 dk kaldı');
    });

    test('expired returns null', () {
      expect(remainingLabel(now - 61 * 60 * 1000, now), isNull);
      expect(remainingLabel(now - 60 * 60 * 1000, now), isNull);
    });
  });
}
