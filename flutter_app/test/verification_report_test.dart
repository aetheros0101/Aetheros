import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/agent/verification.dart';

void main() {
  test('allPassed requires every check passed', () {
    const r = VerificationReport(items: [
      VerificationCheck(id: '1', title: 'fmt', outcome: VerificationOutcome.passed),
      VerificationCheck(id: '2', title: 'test', outcome: VerificationOutcome.failed),
    ]);
    expect(r.allPassed, false);

    const ok = VerificationReport(items: [
      VerificationCheck(id: '1', title: 'fmt', outcome: VerificationOutcome.passed),
    ]);
    expect(ok.allPassed, true);
  });
}
