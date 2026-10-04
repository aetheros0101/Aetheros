import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/core/audit_format.dart';

void main() {
  test('ToolInvoked çıktı özeti okunur', () {
    const j =
        '{"ToolInvoked":{"tool_name":"terminal","success":true,"error":null,'
        '"arguments":["ls"],"output":{"bytes":12,"sha256":"ab","preview":"a.txt b.txt"}}}';
    final o = parseOutputPreview(j)!;
    expect(o.bytes, 12);
    expect(o.preview, 'a.txt b.txt');
  });

  test('çıktısız çağrı ve diğer olaylar null döner', () {
    expect(
        parseOutputPreview(
            '{"ToolInvoked":{"tool_name":"t","success":false,"error":"x","arguments":[],"output":null}}'),
        isNull);
    expect(parseOutputPreview('"ExecutionCompleted"'), isNull);
    expect(parseOutputPreview('{"ExecutionFailed":{"error":"e"}}'), isNull);
    expect(parseOutputPreview('bozuk json'), isNull);
    expect(parseOutputPreview(''), isNull);
  });

  test('outputLabel', () {
    expect(outputLabel(const OutputPreview(bytes: 0, preview: '')),
        contains('Çıktı yok'));
    expect(outputLabel(const OutputPreview(bytes: 5, preview: 'merhaba')),
        '5 bayt · merhaba');
    expect(outputLabel(const OutputPreview(bytes: 5, preview: '  ')),
        '5 bayt çıktı');
  });

  test('auditTitle ve auditIsProblem', () {
    expect(auditTitle('execution_paused'), 'Onay bekliyor');
    expect(auditTitle('bilinmeyen'), 'bilinmeyen');
    expect(auditIsProblem('execution_failed', ''), isTrue);
    expect(auditIsProblem('tool_invoked', "'terminal' başarısız: x"), isTrue);
    expect(auditIsProblem('tool_invoked', "'terminal' çalıştı"), isFalse);
    expect(auditIsProblem('governor_decision', "'terminal' → deny (x)"), isTrue);
    expect(auditIsProblem('governor_decision', "'terminal' → allow"), isFalse);
  });

  test('filtreler', () {
    expect(auditMatches(AuditFilter.all, 'x', ''), isTrue);
    expect(auditMatches(AuditFilter.problems, 'execution_failed', ''), isTrue);
    expect(auditMatches(AuditFilter.problems, 'execution_completed', ''), isFalse);
    expect(auditMatches(AuditFilter.tools, 'tool_invoked', ''), isTrue);
    expect(auditMatches(AuditFilter.tools, 'execution_paused', ''), isFalse);
    expect(auditMatches(AuditFilter.approvals, 'execution_resumed', ''), isTrue);
    expect(auditMatches(AuditFilter.approvals, 'tool_invoked', ''), isFalse);
  });

  test('shortId', () {
    expect(shortId('0123456789abcdef'), '01234567');
    expect(shortId('abc'), 'abc');
  });
}
