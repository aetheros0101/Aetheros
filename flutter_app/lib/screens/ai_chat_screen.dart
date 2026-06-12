// ============================================================
// flutter_app/lib/screens/ai_chat_screen.dart
// Sprint 4 — Gemini AI Chat ekranı
// ============================================================

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/gemini_service.dart';
import 'ai_settings_screen.dart';
import 'script_editor_screen.dart' show ScriptEditorScreen;

class AiChatScreen extends StatefulWidget {
  const AiChatScreen({super.key});
  @override
  State<AiChatScreen> createState() => _AiChatScreenState();
}

class _AiChatScreenState extends State<AiChatScreen> {
  final _inputCtrl  = TextEditingController();
  final _scrollCtrl = ScrollController();
  final List<ChatMessage> _messages = [];

  String _apiKey = '';
  String _model  = 'gemini-2.0-flash';
  bool   _loading = false;
  bool   _checkedKey = false;

  @override
  void initState() {
    super.initState();
    _loadSettings();
  }

  @override
  void dispose() {
    _inputCtrl.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadSettings() async {
    final s = await loadAiSettings();
    setState(() {
      _apiKey = s.apiKey;
      _model  = s.model;
      _checkedKey = true;
    });
  }

  Future<void> _send([String? quickPrompt]) async {
    final text = (quickPrompt ?? _inputCtrl.text).trim();
    if (text.isEmpty || _loading) return;

    if (_apiKey.isEmpty) {
      _goToSettings();
      return;
    }

    final userMsg = ChatMessage(
        role: MessageRole.user, text: text, timestamp: DateTime.now());

    setState(() {
      _messages.add(userMsg);
      _inputCtrl.clear();
      _loading = true;
    });
    _scrollToBottom();

    try {
      final reply = await GeminiService.chat(
        prompt:  text,
        apiKey:  _apiKey,
        model:   _model,
        history: _messages
            .where((m) => m != userMsg)
            .toList(), // önceki mesajlar (son hariç)
      );

      setState(() {
        _messages.add(ChatMessage(
            role: MessageRole.model, text: reply, timestamp: DateTime.now()));
      });
    } on GeminiException catch (e) {
      setState(() {
        _messages.add(ChatMessage(
            role: MessageRole.model,
            text: '⚠️ ${e.message}',
            timestamp: DateTime.now()));
      });
    } catch (e) {
      setState(() {
        _messages.add(ChatMessage(
            role: MessageRole.model,
            text: '⚠️ Bağlantı hatası: $e',
            timestamp: DateTime.now()));
      });
    } finally {
      setState(() => _loading = false);
      _scrollToBottom();
    }
  }

  void _scrollToBottom() {
    Future.delayed(const Duration(milliseconds: 100), () {
      if (_scrollCtrl.hasClients) {
        _scrollCtrl.animateTo(
          _scrollCtrl.position.maxScrollExtent,
          duration: const Duration(milliseconds: 250),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Future<void> _goToSettings() async {
    await Navigator.push(context,
        MaterialPageRoute(builder: (_) => const AiSettingsScreen()));
    _loadSettings();
  }

  void _clearChat() {
    setState(() => _messages.clear());
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F0F1A),
        title: Row(children: [
          const Text('AI Asistan',
              style: TextStyle(color: Colors.white)),
          const SizedBox(width: 8),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: const Color(0xFF4CAF50).withOpacity(0.15),
              borderRadius: BorderRadius.circular(4),
            ),
            child: const Text('Gemini',
                style: TextStyle(
                    color: Color(0xFF4CAF50),
                    fontSize: 10, fontWeight: FontWeight.bold)),
          ),
        ]),
        iconTheme: const IconThemeData(color: Colors.white70),
        actions: [
          IconButton(
            icon: const Icon(Icons.delete_outline,
                color: Colors.white38, size: 20),
            onPressed: _messages.isEmpty ? null : _clearChat,
          ),
          IconButton(
            icon: const Icon(Icons.settings_outlined,
                color: Colors.white70, size: 20),
            onPressed: _goToSettings,
          ),
        ],
      ),
      body: Column(children: [
        // ── API key uyarısı ─────────────────────────
        if (_checkedKey && _apiKey.isEmpty)
          _ApiKeyWarning(onSetup: _goToSettings),

        // ── Mesaj listesi ───────────────────────────
        Expanded(
          child: _messages.isEmpty
              ? _EmptyChat(onQuickPrompt: _send)
              : ListView.builder(
                  controller: _scrollCtrl,
                  padding: const EdgeInsets.all(16),
                  itemCount: _messages.length + (_loading ? 1 : 0),
                  itemBuilder: (_, i) {
                    if (i == _messages.length) return const _TypingIndicator();
                    return _MessageBubble(message: _messages[i]);
                  },
                ),
        ),

        // ── Girdi alanı ─────────────────────────────
        _InputBar(
          controller: _inputCtrl,
          loading: _loading,
          onSend: () => _send(),
        ),
      ]),
    );
  }
}

// ── API key uyarısı ───────────────────────────────────────

class _ApiKeyWarning extends StatelessWidget {
  final VoidCallback onSetup;
  const _ApiKeyWarning({required this.onSetup});

  @override
  Widget build(BuildContext context) => Container(
    width: double.infinity,
    padding: const EdgeInsets.all(12),
    color: const Color(0xFFFFB74D).withOpacity(0.08),
    child: Row(children: [
      const Icon(Icons.warning_amber, color: Color(0xFFFFB74D), size: 18),
      const SizedBox(width: 8),
      const Expanded(child: Text(
        'Gemini API anahtarı ayarlanmadı',
        style: TextStyle(color: Color(0xFFFFB74D), fontSize: 12),
      )),
      TextButton(
        onPressed: onSetup,
        style: TextButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 8)),
        child: const Text('Ayarla',
            style: TextStyle(
                color: Color(0xFFFFB74D),
                fontWeight: FontWeight.bold, fontSize: 12)),
      ),
    ]),
  );
}

// ── Boş sohbet ─────────────────────────────────────────────

class _EmptyChat extends StatelessWidget {
  final ValueChanged<String> onQuickPrompt;
  const _EmptyChat({required this.onQuickPrompt});

  static const _prompts = [
    ('💡', 'WASM modülü nedir, nasıl çalışır?'),
    ('📝', 'Basit bir toplama fonksiyonu için WAT kodu yaz'),
    ('⚙️', 'İki adımlı bir workflow JSON örneği üret'),
    ('🔧', 'AetherOS task önceliklerini açıkla'),
  ];

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Column(mainAxisSize: MainAxisSize.min, children: [
        Container(
          width: 64, height: 64,
          decoration: BoxDecoration(
            gradient: const LinearGradient(
                colors: [Color(0xFF6C63FF), Color(0xFF3F51B5)],
                begin: Alignment.topLeft, end: Alignment.bottomRight),
            borderRadius: BorderRadius.circular(16),
          ),
          child: const Icon(Icons.auto_awesome,
              color: Colors.white, size: 32),
        ),
        const SizedBox(height: 16),
        const Text('AetherOS AI Asistanı',
            style: TextStyle(color: Colors.white,
                fontSize: 16, fontWeight: FontWeight.bold)),
        const SizedBox(height: 6),
        const Text(
          'WASM, workflow ve otomasyon hakkında soru sor',
          style: TextStyle(color: Colors.white38, fontSize: 12),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 24),
        ..._prompts.map((p) => Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: GestureDetector(
            onTap: () => onQuickPrompt(p.$2),
            child: Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: const Color(0xFF1A1A2E),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: Colors.white12),
              ),
              child: Row(children: [
                Text(p.$1, style: const TextStyle(fontSize: 16)),
                const SizedBox(width: 10),
                Expanded(child: Text(p.$2,
                    style: const TextStyle(
                        color: Colors.white70, fontSize: 12))),
              ]),
            ),
          ),
        )),
      ]),
    ),
  );
}

// ── Mesaj balonu ───────────────────────────────────────────

class _MessageBubble extends StatelessWidget {
  final ChatMessage message;
  const _MessageBubble({required this.message});

  bool get _isUser => message.role == MessageRole.user;

  @override
  Widget build(BuildContext context) => Align(
    alignment: _isUser ? Alignment.centerRight : Alignment.centerLeft,
    child: Container(
      margin: const EdgeInsets.only(bottom: 12),
      constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.8),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: _isUser
            ? const Color(0xFF6C63FF)
            : const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.only(
          topLeft: const Radius.circular(14),
          topRight: const Radius.circular(14),
          bottomLeft: Radius.circular(_isUser ? 14 : 4),
          bottomRight: Radius.circular(_isUser ? 4 : 14),
        ),
        border: _isUser ? null : Border.all(color: Colors.white12),
      ),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        SelectableText(message.text,
            style: TextStyle(
                color: _isUser ? Colors.white : Colors.white70,
                fontSize: 13, height: 1.4)),
        const SizedBox(height: 4),
        Row(mainAxisSize: MainAxisSize.min, children: [
          Text(_timeStr(message.timestamp),
              style: TextStyle(
                  color: (_isUser ? Colors.white : Colors.white38)
                      .withOpacity(0.5),
                  fontSize: 10)),
          if (!_isUser) ...[
            const SizedBox(width: 8),
            GestureDetector(
              onTap: () {
                Clipboard.setData(ClipboardData(text: message.text));
                ScaffoldMessenger.of(context).showSnackBar(const SnackBar(
                  content: Text('Kopyalandı'),
                  backgroundColor: Color(0xFF6C63FF),
                  duration: Duration(seconds: 1),
                ));
              },
              child: const Icon(Icons.copy,
                  color: Colors.white24, size: 12),
            ),
          ],
        ]),
      ]),
    ),
  );

  String _timeStr(DateTime t) =>
      '${t.hour.toString().padLeft(2,'0')}:${t.minute.toString().padLeft(2,'0')}';
}

// ── Yazıyor göstergesi ─────────────────────────────────────

class _TypingIndicator extends StatefulWidget {
  const _TypingIndicator();
  @override
  State<_TypingIndicator> createState() => _TypingIndicatorState();
}

class _TypingIndicatorState extends State<_TypingIndicator>
    with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(
        duration: const Duration(milliseconds: 1200), vsync: this)
      ..repeat();
  }

  @override
  void dispose() { _ctrl.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.centerLeft,
    child: Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: const Color(0xFF1A1A2E),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: Colors.white12),
      ),
      child: Row(mainAxisSize: MainAxisSize.min,
          children: List.generate(3, (i) => AnimatedBuilder(
        animation: _ctrl,
        builder: (_, __) {
          final t = (_ctrl.value - i * 0.2) % 1.0;
          final scale = 0.5 + 0.5 * (1 - (t - 0.5).abs() * 2).clamp(0, 1);
          return Padding(
            padding: const EdgeInsets.symmetric(horizontal: 2),
            child: Transform.scale(
              scale: scale,
              child: Container(
                width: 6, height: 6,
                decoration: const BoxDecoration(
                  shape: BoxShape.circle, color: Color(0xFF6C63FF)),
              ),
            ),
          );
        },
      ))),
    ),
  );
}

// ── Girdi çubuğu ───────────────────────────────────────────

class _InputBar extends StatelessWidget {
  final TextEditingController controller;
  final bool loading;
  final VoidCallback onSend;

  const _InputBar({
    required this.controller,
    required this.loading,
    required this.onSend,
  });

  @override
  Widget build(BuildContext context) => Container(
    padding: EdgeInsets.fromLTRB(
        12, 10, 12, 10 + MediaQuery.of(context).padding.bottom),
    decoration: const BoxDecoration(
      color: Color(0xFF0F0F1A),
      border: Border(top: BorderSide(color: Colors.white12)),
    ),
    child: Row(children: [
      Expanded(
        child: TextField(
          controller: controller,
          enabled: !loading,
          style: const TextStyle(color: Colors.white, fontSize: 13),
          minLines: 1,
          maxLines: 4,
          textInputAction: TextInputAction.send,
          onSubmitted: (_) => onSend(),
          decoration: InputDecoration(
            hintText: 'Bir soru sor...',
            hintStyle: const TextStyle(color: Colors.white24),
            filled: true,
            fillColor: const Color(0xFF1A1A2E),
            contentPadding:
                const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(22),
                borderSide: BorderSide.none),
          ),
        ),
      ),
      const SizedBox(width: 8),
      GestureDetector(
        onTap: loading ? null : onSend,
        child: Container(
          width: 42, height: 42,
          decoration: BoxDecoration(
            color: loading
                ? Colors.white12
                : const Color(0xFF6C63FF),
            shape: BoxShape.circle,
          ),
          child: loading
              ? const Padding(
                  padding: EdgeInsets.all(11),
                  child: CircularProgressIndicator(
                      strokeWidth: 2, color: Colors.white38),
                )
              : const Icon(Icons.send, color: Colors.white, size: 18),
        ),
      ),
    ]),
  );
}
