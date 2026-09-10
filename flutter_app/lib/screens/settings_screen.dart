import 'package:flutter/material.dart';
import '../api/aetheros_api.dart';
import '../src/rust/api/aetheros.dart' as rust;
import 'ai_settings_screen.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});
  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  rust.RuntimeInfo? _runtimeInfo;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadRuntime();
  }

  Future<void> _loadRuntime() async {
    try {
      final info = await AetherApi.getRuntimeInfo();
      if (!mounted) return;
      setState(() {
        _runtimeInfo = info;
        _loading = false;
      });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _openAiSettings() async {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const AiSettingsScreen()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0D0D1A),
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: const Text(
          'Ayarlar',
          style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
        ),
        iconTheme: const IconThemeData(color: Colors.white70),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const _SectionHeader(label: 'Runtime Bilgisi', icon: Icons.memory),
          _RuntimeCard(info: _runtimeInfo, loading: _loading),
          const SizedBox(height: 24),
          const _SectionHeader(label: 'Yapay Zeka', icon: Icons.auto_awesome),
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: const Color(0xFF1A1A2E),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.white12),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Anthropic, OpenAI, Gemini ve Ollama ayarlarını tek yerden yönet. API anahtarları güvenli depolamada tutulur.',
                  style: TextStyle(color: Colors.white60, fontSize: 12, height: 1.5),
                ),
                const SizedBox(height: 12),
                SizedBox(
                  width: double.infinity,
                  child: FilledButton.icon(
                    onPressed: _openAiSettings,
                    icon: const Icon(Icons.tune, size: 18),
                    label: const Text('AI Provider Ayarları'),
                    style: FilledButton.styleFrom(
                      backgroundColor: const Color(0xFF6C63FF),
                      padding: const EdgeInsets.symmetric(vertical: 13),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          Center(
            child: Text(
              'AetherOS v${_runtimeInfo?.version ?? "0.1.0"}',
              style: const TextStyle(color: Colors.white24, fontSize: 12),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String label;
  final IconData icon;
  const _SectionHeader({required this.label, required this.icon});

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: Row(
          children: [
            Icon(icon, color: const Color(0xFF6C63FF), size: 17),
            const SizedBox(width: 8),
            Text(
              label,
              style: const TextStyle(
                color: Colors.white70,
                fontSize: 13,
                fontWeight: FontWeight.w600,
                letterSpacing: 0.8,
              ),
            ),
          ],
        ),
      );
}

class _RuntimeCard extends StatelessWidget {
  final rust.RuntimeInfo? info;
  final bool loading;
  const _RuntimeCard({required this.info, required this.loading});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white12),
      ),
      child: loading
          ? const SizedBox(
              height: 24,
              child: Center(
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: Color(0xFF6C63FF),
                ),
              ),
            )
          : info == null
              ? const Text(
                  'Runtime bilgisi alınamadı.',
                  style: TextStyle(color: Colors.white38, fontSize: 12),
                )
              : Wrap(
                  spacing: 16,
                  runSpacing: 8,
                  children: [
                    _InfoChip('Versiyon', info!.version),
                    _InfoChip('Backend', info!.backend),
                    _InfoChip('Worker', '${info!.workerCount}'),
                    _InfoChip(
                      'Durum',
                      info!.isRunning ? 'Aktif' : 'Durdu',
                      color: info!.isRunning
                          ? const Color(0xFF4CAF50)
                          : const Color(0xFFFF5252),
                    ),
                  ],
                ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  final String label;
  final String value;
  final Color? color;
  const _InfoChip(this.label, this.value, {this.color});

  @override
  Widget build(BuildContext context) => Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text('$label: ', style: const TextStyle(color: Colors.white38, fontSize: 12)),
          Text(
            value,
            style: TextStyle(
              color: color ?? Colors.white70,
              fontSize: 12,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      );
}
