import 'dart:convert';

final class WorkflowValidationResult {
  final List<String> errors;
  const WorkflowValidationResult(this.errors);
  bool get isValid => errors.isEmpty;
}

final class WorkflowValidator {
  const WorkflowValidator._();

  static WorkflowValidationResult validateJson(String source) {
    if (source.trim().isEmpty) return const WorkflowValidationResult(['Workflow JSON boş.']);
    dynamic decoded;
    try {
      decoded = jsonDecode(source);
    } on FormatException catch (e) {
      return WorkflowValidationResult(['Geçersiz JSON: ${e.message}']);
    }
    if (decoded is! Map) return const WorkflowValidationResult(['Workflow kökü JSON object olmalı.']);

    final errors = <String>[];
    final name = decoded['name'];
    if (name is! String || name.trim().isEmpty) errors.add('"name" zorunlu ve metin olmalı.');
    final version = decoded['version'];
    if (version != null && version is! String) errors.add('"version" metin olmalı.');
    final steps = decoded['steps'];
    if (steps is! List || steps.isEmpty) {
      errors.add('"steps" en az bir adım içeren liste olmalı.');
    } else {
      final ids = <String>{};
      for (var i = 0; i < steps.length; i++) {
        final step = steps[i];
        if (step is! Map) {
          errors.add('steps[$i] object olmalı.');
          continue;
        }
        final id = step['id'];
        if (id is! String || id.trim().isEmpty) errors.add('steps[$i].id zorunlu.');
        if (id is String && !ids.add(id)) errors.add('Tekrarlanan step id: $id');
        final type = step['type'];
        if (type is! String || type.trim().isEmpty) errors.add('steps[$i].type zorunlu.');
        final deps = step['depends_on'];
        if (deps != null && deps is! List) errors.add('steps[$i].depends_on liste olmalı.');
        if (deps is List) {
          for (final dep in deps) {
            if (dep is! String || dep.isEmpty) errors.add('steps[$i].depends_on yalnızca metin id içermeli.');
          }
        }
      }
      final stepIds = steps.whereType<Map>().map((s) => s['id']).whereType<String>().toSet();
      for (var i = 0; i < steps.length; i++) {
        final deps = steps[i] is Map ? steps[i]['depends_on'] : null;
        if (deps is List) {
          for (final dep in deps.whereType<String>()) {
            if (!stepIds.contains(dep)) errors.add('steps[$i] bilinmeyen bağımlılık: $dep');
          }
        }
      }
    }
    return WorkflowValidationResult(List.unmodifiable(errors));
  }
}
