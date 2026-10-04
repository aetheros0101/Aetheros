import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/terminal_format.dart';

void main() {
  test('parseVerdict bilinen değerleri çözer, bilinmeyeni deny sayar', () {
    expect(parseVerdict('allow'), TerminalVerdict.allow);
    expect(parseVerdict('ask'), TerminalVerdict.ask);
    expect(parseVerdict('deny'), TerminalVerdict.deny);
    expect(parseVerdict('???'), TerminalVerdict.deny);
    expect(parseVerdict(''), TerminalVerdict.deny);
  });

  test('limitLines kısa çıktıya dokunmaz, uzunu kırpar', () {
    expect(limitLines('a\nb', maxLines: 5), 'a\nb');
    final long = List.generate(10, (i) => 'l$i').join('\n');
    final cut = limitLines(long, maxLines: 3);
    expect(cut, 'l0\nl1\nl2\n(… 7 satır daha)');
  });

  test('hızlı komutlar boş değil ve boşluk içermez', () {
    expect(kQuickCommands, isNotEmpty);
    for (final c in kQuickCommands) {
      expect(c.contains(' '), isFalse);
    }
  });
}
