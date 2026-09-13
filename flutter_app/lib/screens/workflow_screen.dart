// ============================================================
// flutter_app/lib/screens/workflow_screen.dart
//
// Faz-1 UI — Workflow yönetimi
//   • Yeni workflow oluştur (adım ekleyerek)
//   • Çalışan/tamamlanan workflow'ları listele
//   • Workflow detayı (durum, adımlar, süre)
// ============================================================

import 'dart:convert';
import '../core/lifecycle_poller.dart';
import '../core/app_error.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;
import '../core/app_theme.dart';

class WorkflowScreen extends StatefulWidget {
  const WorkflowScreen({super.key});
  @override
  State<WorkflowScreen> createState() => _WorkflowScreenState();
}

class _WorkflowScreenState extends State<WorkflowScreen> {
  List<rust.WorkflowStatusResponse> _workflows = [];
  bool   _loading = true;
  late final LifecyclePoller _poller;

  // Rust tarafı bir workflow'un orijinal adım tanımını geri döndürmüyor
  // (WorkflowStatusResponse sadece durum metadata'sı taşıyor) — bu yüzden
  // "yeniden çalıştır" için adımları burada, uygulama oturumu boyunca
  // client-side önbelleğe alıyoruz. NOT: Uygulama yeniden başlatılırsa bu
  // önbellek sıfırlanır; o durumda eski bir workflow retry edilemez,
  // yeniden oluşturulması gerekir.
  final Map<String, List<rust.WorkflowStepRequest>> _stepsCache = {};

  @override
  void initState() {
    super.initState();
    _restoreStepCache();
    _load();
    _poller = LifecyclePoller(interval: const Duration(seconds: 3), onTick: () => _load(silent: true));
    _poller.start();
  }

  @override
  void dispose() { _poller.stop(); super.dispose(); }

  Future<void> _restoreStepCache() async {
    final prefs = await SharedPreferences.getInstance();
    for (final key in prefs.getKeys().where((k) => k.startsWith('workflow_steps_'))) {
      final workflowId = key.substring('workflow_steps_'.length);
      final raw = prefs.getStringList(key) ?? const [];
      final steps = <rust.WorkflowStepRequest>[];
      for (final value in raw) {
        try {
          final m = jsonDecode(value) as Map<String, dynamic>;
          steps.add(rust.WorkflowStepRequest(
            id: m['id'].toString(), name: m['name'].toString(),
            kind: m['kind'].toString(), entrypoint: m['entrypoint']?.toString(),
            dependsOn: (m['dependsOn'] as List? ?? const []).map((e) => e.toString()).toList(),
            retryable: m['retryable'] == true,
          ));
        } catch (_) {}
      }
      if (steps.isNotEmpty) _stepsCache[workflowId] = steps;
    }
  }

  Future<void> _persistStepCache(String workflowId, List<rust.WorkflowStepRequest> steps) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setStringList('workflow_steps_$workflowId', steps.map((s) => jsonEncode({
      'id': s.id, 'name': s.name, 'kind': s.kind, 'entrypoint': s.entrypoint,
      'dependsOn': s.dependsOn, 'retryable': s.retryable,
    })).toList());
  }

  Future<void> _load({bool silent = false}) async {
    if (!silent && mounted) setState(() => _loading = true);
    try {
      final wf = await AetherApi.listWorkflows(limit: 50);
      if (mounted) setState(() { _workflows = wf; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _onWorkflowCreated(rust.WorkflowStartResponse resp, List<rust.WorkflowStepRequest> steps) {
    _stepsCache[resp.workflowId] = steps;
    _persistStepCache(resp.workflowId, steps);
    _load();
  }

  Future<void> _retry(rust.WorkflowStatusResponse wf) async {
    final steps = _stepsCache[wf.workflowId];
    if (steps == null) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
        content: Text('Bu workflow için adım bilgisi bulunamadı '
            '(uygulama yeniden başlatılmış olabilir) — yeni workflow oluştur.'),
        backgroundColor: Colors.redAccent,
      ));
      return;
    }
    try {
      final resp = await AetherApi.startWorkflow(name: wf.name, steps: steps);
      _stepsCache[resp.workflowId] = steps;
      await _persistStepCache(resp.workflowId, steps);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
        content: Text('Workflow yeniden başlatıldı'),
        backgroundColor: Color(0xFFE8A838),
      ));
      _load();
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text('Yeniden çalıştırılamadı: $e'),
        backgroundColor: Colors.redAccent,
      ));
    }
  }

  void _showBuilder() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: AetherColors.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _WorkflowBuilderSheet(onCreated: _onWorkflowCreated),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AetherColors.background,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text('Workflow Yönetimi',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh, color: Colors.white70),
            onPressed: _load,
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _showBuilder,
        backgroundColor: const Color(0xFFE8A838),
        icon: const Icon(Icons.account_tree, color: Colors.white),
        label: const Text('Yeni Workflow', style: TextStyle(color: Colors.white)),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFFE8A838)))
          : _workflows.isEmpty
              ? _EmptyState(onNew: _showBuilder)
              : ListView.builder(
                  padding: const EdgeInsets.fromLTRB(12, 8, 12, 100),
                  itemCount: _workflows.length,
                  itemBuilder: (_, i) => _WorkflowCard(
                    workflow: _workflows[i],
                    onTap: () => _showDetail(_workflows[i]),
                    onRetry: () => _retry(_workflows[i]),
                  ),
                ),
    );
  }

  void _showDetail(rust.WorkflowStatusResponse wf) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: AetherColors.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _WorkflowDetailSheet(
        workflow: wf,
        onRetry: () {
          Navigator.pop(context);
          _retry(wf);
        },
      ),
    );
  }
}

// ── Workflow Builder ──────────────────────────────────────────

class _WorkflowStep {
  String id;
  String name;
  String kind;   // wasm | agent | ai | task
  String entrypoint;
  List<String> dependsOn;
  bool retryable;

  _WorkflowStep({
    required this.id,
    required this.name,
    this.kind       = 'wasm',
    this.entrypoint = 'run',
    this.dependsOn  = const [],
    this.retryable  = false,
  });
}

class _WorkflowBuilderSheet extends StatefulWidget {
  final void Function(rust.WorkflowStartResponse, List<rust.WorkflowStepRequest>) onCreated;
  const _WorkflowBuilderSheet({required this.onCreated});
  @override State<_WorkflowBuilderSheet> createState() => _WorkflowBuilderSheetState();
}

class _WorkflowBuilderSheetState extends State<_WorkflowBuilderSheet> {
  final _nameCtrl = TextEditingController(text: 'my-workflow');
  final List<_WorkflowStep> _steps = [];
  bool   _submitting = false;
  String? _error;
  int    _stepCounter = 1;

  @override
  void dispose() { _nameCtrl.dispose(); super.dispose(); }

  void _addStep() {
    setState(() => _steps.add(_WorkflowStep(
      id:   's$_stepCounter',
      name: 'Adım $_stepCounter',
    )));
    _stepCounter++;
  }

  void _removeStep(int index) {
    setState(() {
      final removedId = _steps[index].id;
      _steps.removeAt(index);
      // Silinen adıma bağlı kalan tüm bağımlılıkları temizle; aksi halde
      // backend'e artık var olmayan bir step id'si gönderilir.
      for (final step in _steps) {
        step.dependsOn = step.dependsOn
            .where((id) => id != removedId)
            .toList(growable: false);
      }
    });
  }

  Future<void> _submit() async {
    if (_nameCtrl.text.trim().isEmpty) {
      setState(() => _error = 'Workflow adı boş olamaz.');
      return;
    }
    if (_steps.isEmpty) {
      setState(() => _error = 'En az bir adım eklemeden workflow çalıştırılamaz.');
      return;
    }
    setState(() { _submitting = true; _error = null; });

    final steps = _steps.map((s) => rust.WorkflowStepRequest(
      id:         s.id,
      name:       s.name,
      kind:       s.kind,
      entrypoint: s.entrypoint.isEmpty ? null : s.entrypoint,
      dependsOn:  s.dependsOn,
      retryable:  s.retryable,
    )).toList();

    try {
      final resp = await AetherApi.startWorkflow(name: _nameCtrl.text.trim(), steps: steps);
      if (mounted) {
        Navigator.pop(context);
        widget.onCreated(resp, steps);
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
          content: Text('Workflow başlatıldı'),
          backgroundColor: Color(0xFFE8A838),
        ));
      }
    } catch (e) {
      if (!mounted) return;
      setState(() { _submitting = false; _error = userFacingError(e); });
    }
  }

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.75,
      maxChildSize: 0.95,
      builder: (_, ctrl) => Padding(
        padding: EdgeInsets.only(bottom: MediaQuery.of(context).viewInsets.bottom),
        child: ListView(controller: ctrl, padding: const EdgeInsets.all(20), children: [
          // Handle
          Center(child: Container(width: 40, height: 4,
              margin: const EdgeInsets.only(bottom: 16),
              decoration: BoxDecoration(color: Colors.white24,
                  borderRadius: BorderRadius.circular(2)))),

          const Text('Yeni Workflow',
              style: TextStyle(color: Colors.white,
                  fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 16),

          // Ad
          TextField(
            controller: _nameCtrl,
            style: const TextStyle(color: Colors.white),
            decoration: _inputDeco('Workflow adı', 'my-workflow'),
          ),
          const SizedBox(height: 20),

          // Adımlar başlık
          Row(children: [
            const Text('Adımlar', style: TextStyle(
                color: Colors.white70, fontSize: 15, fontWeight: FontWeight.w600)),
            const Spacer(),
            TextButton.icon(
              onPressed: _addStep,
              icon: const Icon(Icons.add, size: 16, color: Color(0xFFE8A838)),
              label: const Text('Adım Ekle',
                  style: TextStyle(color: Color(0xFFE8A838))),
            ),
          ]),

          if (_steps.isEmpty)
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: const Color(0xFF252540),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: Colors.white12),
              ),
              child: const Text(
                '"Adım Ekle" ile başla. Adımlar sırayla çalışır.\n'
                'Bağımlılık ekleyerek paralel akış kurabilirsin.',
                style: TextStyle(color: Colors.white38, fontSize: 12),
                textAlign: TextAlign.center,
              ),
            )
          else
            ...List.generate(_steps.length, (i) =>
                _StepCard(
                  step: _steps[i],
                  index: i,
                  availableIds: _steps.sublist(0, i).map((s) => s.id).toList(),
                  onRemove: () => _removeStep(i),
                  onChanged: () => setState(() {}),
                )),

          if (_error != null) ...[
            const SizedBox(height: 12),
            Text(_error!, style: const TextStyle(color: Color(0xFFFF5252), fontSize: 12)),
          ],
          const SizedBox(height: 20),

          SizedBox(width: double.infinity,
            child: ElevatedButton.icon(
              onPressed: _submitting ? null : _submit,
              icon: _submitting
                  ? const SizedBox(width: 16, height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.play_arrow, color: Colors.white),
              label: Text(_submitting ? 'Başlatılıyor…' : 'Çalıştır',
                  style: const TextStyle(color: Colors.white, fontSize: 16)),
              style: ElevatedButton.styleFrom(
                backgroundColor: const Color(0xFFE8A838),
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ),
        ]),
      ),
    );
  }
}

// ── Adım kartı (builder içi) ──────────────────────────────────

class _StepCard extends StatelessWidget {
  final _WorkflowStep step;
  final int index;
  final List<String> availableIds;
  final VoidCallback onRemove;
  final VoidCallback onChanged;
  const _StepCard({required this.step, required this.index,
      required this.availableIds, required this.onRemove,
      required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF252540),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: Colors.white12),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        // Başlık
        Row(children: [
          Container(
            width: 24, height: 24,
            decoration: BoxDecoration(
              color: const Color(0xFFE8A838).withOpacity(0.2),
              shape: BoxShape.circle,
            ),
            child: Center(child: Text('${index + 1}',
                style: const TextStyle(color: Color(0xFFE8A838),
                    fontSize: 11, fontWeight: FontWeight.bold))),
          ),
          const SizedBox(width: 8),
          Expanded(child: TextFormField(
            initialValue: step.name,
            style: const TextStyle(color: Colors.white, fontSize: 13),
            decoration: const InputDecoration(
              hintText: 'Adım adı',
              hintStyle: TextStyle(color: Colors.white38),
              border: InputBorder.none,
              isDense: true,
            ),
            onChanged: (v) { step.name = v; onChanged(); },
          )),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white38, size: 18),
            onPressed: onRemove,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
          ),
        ]),
        const Divider(color: Colors.white12),

        // Tür seçimi
        Row(children: [
          const Text('Tür:', style: TextStyle(color: Colors.white54, fontSize: 12)),
          const SizedBox(width: 8),
          ...['wasm', 'agent', 'ai', 'task'].map((k) => _KindChip(
            label: k,
            selected: step.kind == k,
            onTap: () { step.kind = k; onChanged(); },
          )),
        ]),
        const SizedBox(height: 8),

        // Entrypoint
        TextFormField(
          initialValue: step.entrypoint,
          style: const TextStyle(color: Colors.white, fontSize: 12),
          decoration: const InputDecoration(
            labelText: 'Entrypoint',
            labelStyle: TextStyle(color: Colors.white38, fontSize: 12),
            hintText: 'run',
            hintStyle: TextStyle(color: Colors.white24),
            border: OutlineInputBorder(),
            enabledBorder: OutlineInputBorder(
              borderSide: BorderSide(color: Colors.white12),
            ),
            focusedBorder: OutlineInputBorder(
              borderSide: BorderSide(color: Color(0xFFE8A838)),
            ),
            isDense: true,
            contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          ),
          onChanged: (v) { step.entrypoint = v; onChanged(); },
        ),

        // Bağımlılıklar
        if (availableIds.isNotEmpty) ...[
          const SizedBox(height: 8),
          Wrap(children: [
            const Padding(
              padding: EdgeInsets.only(right: 6, top: 4),
              child: Text('Bekle:', style: TextStyle(color: Colors.white54, fontSize: 12)),
            ),
            ...availableIds.map((id) {
              final selected = step.dependsOn.contains(id);
              return InkWell(
                onTap: () {
                  if (selected) step.dependsOn.remove(id);
                  else step.dependsOn.add(id);
                  onChanged();
                },
                child: Container(
                  margin: const EdgeInsets.only(right: 4, top: 4),
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: selected
                        ? const Color(0xFFE8A838).withOpacity(0.2)
                        : AetherColors.surface,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: selected
                          ? const Color(0xFFE8A838)
                          : Colors.white24,
                    ),
                  ),
                  child: Text(id,
                      style: TextStyle(
                          color: selected
                              ? const Color(0xFFE8A838)
                              : Colors.white54,
                          fontSize: 11)),
                ),
              );
            }),
          ]),
        ],
      ]),
    );
  }
}

// ── Workflow listesi kartı ─────────────────────────────────────

class _WorkflowCard extends StatelessWidget {
  final rust.WorkflowStatusResponse workflow;
  final VoidCallback onTap;
  final VoidCallback onRetry;
  const _WorkflowCard({required this.workflow, required this.onTap, required this.onRetry});

  @override
  Widget build(BuildContext context) {
    final status = workflow.status;
    final color  = _statusColor(status);
    final start  = DateTime.fromMillisecondsSinceEpoch(workflow.startedAt);
    final end    = workflow.finishedAt != null
        ? DateTime.fromMillisecondsSinceEpoch(workflow.finishedAt!)
        : null;
    final dur = end != null
        ? _fmt(end.difference(start))
        : _fmt(DateTime.now().difference(start));

    return InkWell(
      onTap: onTap,
      child: Container(
        margin: const EdgeInsets.only(bottom: 10),
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: AetherColors.surface,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: color.withOpacity(0.3)),
        ),
        child: Row(children: [
          Container(
            width: 40, height: 40,
            decoration: BoxDecoration(
              color: color.withOpacity(0.15),
              shape: BoxShape.circle,
            ),
            child: Icon(Icons.account_tree, color: color, size: 20),
          ),
          const SizedBox(width: 12),
          Expanded(child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(workflow.name,
                  style: const TextStyle(color: Colors.white,
                      fontWeight: FontWeight.bold)),
              Text(workflow.workflowId.substring(0, 8).toUpperCase(),
                  style: const TextStyle(color: Colors.white38,
                      fontSize: 11, fontFamily: 'monospace')),
            ],
          )),
          // Yeniden çalıştır — koşum bitmiş (completed/failed) her workflow
          // için gösterilir; hâlâ çalışan (running) bir tanesini tekrar
          // tetiklemek anlamsız olduğu için sadece bitmişlerde çıkar.
          if (status != 'running')
            IconButton(
              icon: const Icon(Icons.replay, color: Colors.white54, size: 20),
              tooltip: 'Yeniden çalıştır',
              onPressed: onRetry,
            ),
          Column(crossAxisAlignment: CrossAxisAlignment.end, children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
              decoration: BoxDecoration(
                color: color.withOpacity(0.15),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(status,
                  style: TextStyle(color: color, fontSize: 11,
                      fontWeight: FontWeight.w600)),
            ),
            const SizedBox(height: 4),
            Text(dur, style: const TextStyle(color: Colors.white38, fontSize: 11)),
          ]),
        ]),
      ),
    );
  }

  Color _statusColor(String s) => switch (s) {
    'completed' => AetherColors.success,
    'failed'    => const Color(0xFFFF5252),
    'running'   => const Color(0xFFE8A838),
    _           => Colors.white38,
  };

  String _fmt(Duration d) {
    if (d.inSeconds < 60) return '${d.inSeconds}s';
    if (d.inMinutes < 60) return '${d.inMinutes}dk';
    return '${d.inHours}sa';
  }
}

// ── Workflow detay ────────────────────────────────────────────

class _WorkflowDetailSheet extends StatelessWidget {
  final rust.WorkflowStatusResponse workflow;
  final VoidCallback onRetry;
  const _WorkflowDetailSheet({required this.workflow, required this.onRetry});

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.55,
      maxChildSize: 0.9,
      builder: (_, ctrl) => Container(
        padding: const EdgeInsets.all(20),
        child: ListView(controller: ctrl, children: [
          Center(child: Container(width: 40, height: 4,
              margin: const EdgeInsets.only(bottom: 16),
              decoration: BoxDecoration(color: Colors.white24,
                  borderRadius: BorderRadius.circular(2)))),
          const Text('Workflow Detayı',
              style: TextStyle(color: Colors.white,
                  fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 16),
          _Row('Workflow ID', workflow.workflowId),
          _Row('Ad',          workflow.name),
          _Row('Durum',       workflow.status),
          if (workflow.error != null)
            _Row('Hata', workflow.error!, errorColor: true),
          const SizedBox(height: 16),
          if (workflow.status != 'running')
            SizedBox(width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: onRetry,
                icon: const Icon(Icons.replay, size: 18, color: Colors.white),
                label: const Text('Yeniden Çalıştır',
                    style: TextStyle(color: Colors.white)),
                style: ElevatedButton.styleFrom(
                    backgroundColor: const Color(0xFFE8A838)),
              ),
            ),
          const SizedBox(height: 10),
          SizedBox(width: double.infinity,
            child: OutlinedButton.icon(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: workflow.workflowId));
                ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                  content: Text('Workflow ID kopyalandı'),
                  duration: Duration(seconds: 1),
                ));
              },
              icon: const Icon(Icons.copy, size: 16, color: Color(0xFFE8A838)),
              label: const Text('ID Kopyala',
                  style: TextStyle(color: Color(0xFFE8A838))),
              style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: Color(0xFFE8A838))),
            ),
          ),
        ]),
      ),
    );
  }
}

// ── Küçük yardımcılar ─────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  final VoidCallback onNew;
  const _EmptyState({required this.onNew});
  @override
  Widget build(BuildContext context) => Center(child: Column(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      const Icon(Icons.account_tree_outlined, size: 64, color: Colors.white24),
      const SizedBox(height: 16),
      const Text('Henüz workflow yok',
          style: TextStyle(color: Colors.white54, fontSize: 16)),
      const SizedBox(height: 8),
      const Text('Adımları ekleyip bağımlılıklarını seçerek oluştur',
          style: TextStyle(color: Colors.white38, fontSize: 13)),
      const SizedBox(height: 24),
      ElevatedButton.icon(
        onPressed: onNew,
        icon: const Icon(Icons.add, color: Colors.white),
        label: const Text('Yeni Workflow', style: TextStyle(color: Colors.white)),
        style: ElevatedButton.styleFrom(
          backgroundColor: const Color(0xFFE8A838),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        ),
      ),
    ],
  ));
}

class _KindChip extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _KindChip({required this.label, required this.selected, required this.onTap});
  @override
  Widget build(BuildContext context) => InkWell(
    onTap: onTap,
    child: Container(
      margin: const EdgeInsets.only(left: 6),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: selected ? const Color(0xFFE8A838) : AetherColors.surface,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(
          color: selected ? const Color(0xFFE8A838) : Colors.white24),
      ),
      child: Text(label,
          style: TextStyle(
              color: selected ? Colors.white : Colors.white54,
              fontSize: 11, fontWeight: FontWeight.w600)),
    ),
  );
}

class _Row extends StatelessWidget {
  final String label;
  final String value;
  final bool errorColor;
  const _Row(this.label, this.value, {this.errorColor = false});
  @override
  Widget build(BuildContext context) => Container(
    margin: const EdgeInsets.only(bottom: 10),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: const Color(0xFF252540),
      borderRadius: BorderRadius.circular(10),
    ),
    child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
      SizedBox(width: 110,
          child: Text(label,
              style: const TextStyle(color: Colors.white54, fontSize: 13))),
      Expanded(child: Text(value,
          style: TextStyle(
              color: errorColor ? const Color(0xFFFF5252) : Colors.white,
              fontSize: 13))),
    ]),
  );
}

InputDecoration _inputDeco(String label, String hint) => InputDecoration(
  labelText: label,
  hintText: hint,
  labelStyle: const TextStyle(color: Colors.white54),
  hintStyle: const TextStyle(color: Colors.white24),
  enabledBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(10),
    borderSide: const BorderSide(color: Colors.white24),
  ),
  focusedBorder: OutlineInputBorder(
    borderRadius: BorderRadius.circular(10),
    borderSide: const BorderSide(color: Color(0xFFE8A838)),
  ),
  filled: true,
  fillColor: const Color(0xFF252540),
);
