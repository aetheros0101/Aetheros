// ============================================================
// flutter_app/lib/core/chat_to_agent.dart
//
// Chat'ten "Agent ile yap": sohbet mesajını Yeni Agent formunun hedefi
// olarak hazırlar. Chat araçsız KALIR; işi yapacak olan agent'tır ve o,
// kendi yetki/onay zincirinden geçer (kullanıcı formda terminal yetkisini
// görür, isterse kaldırır).
// ============================================================

/// Hedef metni için üst sınır (Yeni Agent formuna sığsın, prompt şişmesin).
const int kMaxObjectiveChars = 500;

/// Mesajı agent hedefine çevirir: baştaki/sondaki boşlukları atar, ardışık
/// boşluk/satır sonlarını tek boşluğa indirir, uzunsa kırpar.
String objectiveFromChat(String text, {int maxChars = kMaxObjectiveChars}) {
  final cleaned = text.trim().replaceAll(RegExp(r'\s+'), ' ');
  if (cleaned.length <= maxChars) return cleaned;
  return cleaned.substring(0, maxChars).trimRight();
}
