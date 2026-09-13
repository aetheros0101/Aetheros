import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/services/workflow_validator.dart';

void main() {
  test('valid workflow passes structural validation', () {
    const source = '''{
      "name": "demo",
      "version": "1.0.0",
      "steps": [
        {"id":"a","type":"wasm_call"},
        {"id":"b","type":"wasm_call","depends_on":["a"]}
      ]
    }''';
    expect(WorkflowValidator.validateJson(source).isValid, isTrue);
  });

  test('duplicate and unknown dependencies are rejected', () {
    const source = '''{
      "name": "demo",
      "steps": [
        {"id":"a","type":"wasm_call"},
        {"id":"a","type":"wasm_call","depends_on":["missing"]}
      ]
    }''';
    final result = WorkflowValidator.validateJson(source);
    expect(result.isValid, isFalse);
    expect(result.errors.join(' '), contains('Tekrarlanan step id'));
    expect(result.errors.join(' '), contains('bilinmeyen bağımlılık'));
  });

  test('non-object root is rejected', () {
    final result = WorkflowValidator.validateJson('[]');
    expect(result.isValid, isFalse);
  });
}
