#!/usr/bin/env python3
"""Agent paneli: takılı kalan onay kartı + biten işin 'çalışıyor' kalması.
 - poll: çözülmüş onaylar listeden düşüyordu ama eski liste korunuyordu
   (`related.isEmpty ? s.approvals : related`) -> kart sonsuza dek 'pending'.
 - approve/reject: yerelde hemen onaylandı/reddedildi işaretlenir.
 - 'denied' (onay reddi / TTL) bitmiş durum sayılır; poll durur, görev
   satırı 'failed' gösterir.
Proje kökünde: python3 fix_agent_poll.py   (idempotent)"""
import pathlib, sys

rel = "flutter_app/lib/application/services/frb_agent_service.dart"
p = pathlib.Path(rel)
if not p.exists():
    sys.exit("HATA: proje kökünde değilsiniz")
t = p.read_text()
if "_markApproval" in t:
    print("Zaten uygulanmış."); sys.exit(0)

def rep(old, new):
    global t
    if t.count(old) != 1:
        sys.exit(f"HATA: kalıp bulunamadı/benzersiz değil:\n{old[:90]}")
    t = t.replace(old, new, 1)

# 1) çözülen onayları listeden düşür (eski kartı tutma)
rep("          approvals: related.isEmpty ? s.approvals : related,\n",
    """          approvals: [
            ...related,
            // Artık bekleyen listede olmayan eski kartlar çözülmüştür.
            for (final old in s.approvals)
              if (!related.any((r) => r.id == old.id))
                old.status == ApprovalStatus.pending
                    ? old.copyWith(status: ApprovalStatus.expired)
                    : old,
          ],
""")

# 2) denied = bitmiş
rep("""      final done = status.status == 'completed' ||
          status.status == 'failed' ||
          status.status == 'cancelled';""",
    """      final done = status.status == 'completed' ||
          status.status == 'failed' ||
          status.status == 'denied' ||
          status.status == 'cancelled';""")

# 3) denied görev satırı
rep("      case 'failed':\n        return const [",
    "      case 'failed':\n      case 'denied':\n        return const [")

# 4) approve/reject: iyimser işaretleme
rep("""  Future<void> approve(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: true);""",
    """  Future<void> approve(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: true);
    _markApproval(approvalId, ApprovalStatus.approved);""")
rep("""  Future<void> reject(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: false);""",
    """  Future<void> reject(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: false);
    _markApproval(approvalId, ApprovalStatus.rejected);
    for (final s in _state.sessions) {
      if (s.approvals.any((a) => a.id == approvalId)) _startPoll(s.id);
    }""")
rep("""  @override
  Future<void> stop(String sessionId) async {""",
    """  void _markApproval(String approvalId, ApprovalStatus status) {
    final sessions = [
      for (final s in _state.sessions)
        s.approvals.any((a) => a.id == approvalId)
            ? s.copyWith(approvals: [
                for (final a in s.approvals)
                  a.id == approvalId ? a.copyWith(status: status) : a,
              ])
            : s,
    ];
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> stop(String sessionId) async {""")
p.write_text(t)
print("Değişti:", rel)
