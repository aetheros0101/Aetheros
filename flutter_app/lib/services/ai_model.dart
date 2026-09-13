enum AiCapability { chat, vision, tools, audioInput, audioOutput, embeddings, imageGeneration }
enum AiModelLifecycle { active, deprecated, unknown }

enum AiModality { text, image, audio, video }

/// Provider-neutral runtime model descriptor. The catalog is authoritative;
/// fields are optional because providers expose different metadata.
class AiModelOption {
  final String value;
  final String label;
  final String providerId;
  final Set<AiCapability> capabilities;
  final Set<AiModality> inputModalities;
  final Set<AiModality> outputModalities;
  final int? contextWindow;
  final int? maxOutputTokens;
  final AiModelLifecycle lifecycle;

  const AiModelOption(
    this.value,
    this.label, {
    this.providerId = '',
    this.capabilities = const {AiCapability.chat},
    this.inputModalities = const {AiModality.text},
    this.outputModalities = const {AiModality.text},
    this.contextWindow,
    this.maxOutputTokens,
    this.lifecycle = AiModelLifecycle.unknown,
  });

  bool supports(AiCapability capability) => capabilities.contains(capability);

  @override
  bool operator ==(Object other) =>
      other is AiModelOption && other.providerId == providerId && other.value == value;

  @override
  int get hashCode => Object.hash(providerId, value);
}
