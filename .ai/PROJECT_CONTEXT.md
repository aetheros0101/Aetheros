# Rust Backend (src/)

> Bu dosya implementasyon gövdesi içermez. Sadece mimari, modül amacı (doc comment) ve public API imzaları yer alır.

## Mimari

| Katman | Modül | LOC | Fan-in | Fan-out |
|---|---|---|---|---|
| 1 | types | 99 | 13 | 0 |
| 1 | task | 771 | 11 | 4 |
| 0 | errors | 163 | 10 | 0 |
| 2 | events | 139 | 7 | 1 |
| 1 | wasm | 896 | 7 | 2 |
| 1 | persistence | 334 | 5 | 3 |
| 1 | runtime | 759 | 5 | 6 |
| 2 | orchestration | 1933 | 4 | 6 |
| 2 | workflows | 1650 | 4 | 7 |
| 0 | agents | 917 | 3 | 3 |
| 1 | metrics | 236 | 3 | 1 |
| 0 | remote | 981 | 3 | 0 |
| 0 | ai | 1302 | 2 | 1 |
| 0 | scripting | 403 | 2 | 5 |
| 0 | security | 670 | 2 | 0 |
| 1 | worker | 773 | 2 | 6 |
| 0 | api | 1731 | 1 | 13 |
| 1 | logging | 622 | 1 | 3 |
| 0 | tests | 3231 | 1 | 14 |
| 0 | bridge | 776 | 0 | 8 |
| 0 | config | 8 | 0 | 0 |
| 0 | plugins | 206 | 0 | 0 |
| 0 | registry | 178 | 0 | 3 |
| 0 | sdk | 240 | 0 | 0 |

**⚠ Döngüsel bağımlılıklar tespit edildi:**

- api → orchestration → runtime → persistence → task → tests → api
- orchestration → runtime → persistence → task → tests → orchestration
- orchestration → runtime → persistence → task → tests → workflows → orchestration
- persistence → task → tests → persistence
- persistence → task → tests → runtime → runtime
- persistence → task → tests → workflows → runtime → runtime
- scripting → task → task → tests
- scripting → wasm → task → task → tests
- task → tests → task
- task → tests → workflows → task

## Modüller

### `agents` (katman 0, 917 LOC)

**Bağımlı olduğu:** ai, errors, types

**Kendisine bağımlı olanlar:** api, scripting, workflows

**Public API:**

- `pub struct AgentExecutionState`
- `pub struct ReasoningTrace`
- `pub struct ToolInvocation`
- `pub struct RegisteredAgent`
- `pub struct AgentRegistry`
- `impl AgentRegistry :: fn new() -> Self`
- `impl AgentRegistry :: fn register(& mut self, agent : RegisteredAgent)`
- `impl AgentRegistry :: fn agents(& self) -> & [RegisteredAgent]`
- `pub struct AgentManager`
- `impl AgentManager :: fn new() -> Self`
- `impl AgentManager :: fn register(& mut self, agent : RegisteredAgent)`
- `impl AgentManager :: fn total_agents(& self) -> usize`
- `impl AgentManager :: fn agent_ids(& self) -> Vec <Uuid>`
- `pub enum AgentState`
- `pub struct AgentMemoryRecord`
- `pub struct AgentMemory`
- `impl AgentMemory :: fn new() -> Self`
- `impl AgentMemory :: fn store(& self, key : impl Into < String >, value : Value)` — Adım sonucunu key-value olarak kaydet.
- `impl AgentMemory :: fn get(& self, key : & str) -> Option <Value>` — Daha önce kaydedilmiş değeri getir.
- `impl AgentMemory :: fn snapshot(& self) -> Vec <(String , Value)>` — Tüm kayıtları döndür (reasoning context için).
- `impl AgentMemory :: fn len(& self) -> usize`
- `impl AgentMemory :: fn is_empty(& self) -> bool`
- `pub struct AgentRuntime`
- `impl AgentRuntime :: fn new(budget : AgentExecutionBudget, tools : Vec < Arc < dyn AgentTool > >) -> Self`
- `impl AgentRuntime :: async fn execute(context : AgentContext, objective : String, budget : AgentExecutionBudget, tools : Vec < Arc < dyn AgentTool > >) -> Result <Uuid , RuntimeError>` — Agent execution döngüsü. objective → plan → adım adım çalıştır → execution_id
- `pub enum AgentError`
- `pub struct AgentExecutionBudget`
- `pub struct AgentContext`
- `pub struct AgentExecutor`
- `impl AgentExecutor :: async fn execute(context : AgentContext, objective : String, budget : AgentExecutionBudget, tools : Vec < Arc < dyn AgentTool > >) -> Result <Uuid , RuntimeError>`
- `pub struct AgentCapacity`
- `pub struct AgentPlanner`
- `impl AgentPlanner :: async fn plan(objective : String) -> AgentPlan` — Objective'den plan üret. AnthropicProvider mevcutsa AI destekli plan (ai_plan). Değilse (API key yok, rate-limit, parse hatası) fallback_plan.
- `pub enum AgentLifecycleState`
- `pub struct AgentLifecycle`
- `impl AgentLifecycle :: fn new() -> Self`
- `impl AgentLifecycle :: fn transition(& mut self, state : AgentState)`
- `impl AgentLifecycle :: fn state(& self) -> AgentState`
- `pub struct AgentCapabilities`
- `pub enum AgentCapability`
- `pub trait AgentPersistence`
- `pub enum AgentCancellation`
- `pub struct AgentSubscription`

### `ai` (katman 0, 1302 LOC)

**Bağımlı olduğu:** types

**Kendisine bağımlı olanlar:** agents, workflows

**Public API:**

- `pub struct EmbeddingVector`
- `pub struct RetrievalResult`
- `pub trait VectorStore`
- `pub struct EmbeddingVector`
- `pub struct ConversationSession`
- `pub struct AiContext`
- `pub struct ContextWindow`
- `pub struct ToolCall`
- `pub struct ToolRegistry`
- `impl ToolRegistry :: fn new() -> Self`
- `impl ToolRegistry :: fn register(& self, tool : Arc < dyn AgentTool >)`
- `impl ToolRegistry :: fn total_tools(& self) -> usize`
- `pub struct PromptTemplate`
- `pub struct PromptVersion`
- `pub struct InferenceRequest`
- `impl InferenceRequest :: fn new(prompt : impl Into < String >, max_tokens : usize) -> Self`
- `impl InferenceRequest :: fn with_system(mut self, system : impl Into < String >) -> Self`
- `impl InferenceRequest :: fn with_temperature(mut self, temperature : f32) -> Self`
- `pub trait AiProvider`
- `pub struct InferenceResponse`
- `pub struct InferenceEngine`
- `impl InferenceEngine :: fn new(provider : Arc < dyn AiProvider >) -> Self`
- `impl InferenceEngine :: async fn infer(& self, request : InferenceRequest) -> Result <InferenceResponse , AiError ,>`
- `pub struct AiRetryPolicy`
- `pub struct RetryPolicy`
- `pub struct RoutingPolicy`
- `pub struct TokenBudgetPolicy`
- `pub struct AiExecutionPlan`
- `pub struct AiExecutionPlan`
- `pub trait AiPlanner`
- `pub struct AiExecutionPlan`
- `pub struct RoutingPolicy`
- `pub struct ProviderRouter`
- `impl ProviderRouter :: fn new() -> Self`
- `impl ProviderRouter :: fn register(& mut self, provider : Arc < dyn AiProvider , >)`
- `impl ProviderRouter :: async fn infer(& self, request : InferenceRequest) -> Result <InferenceResponse , AiError ,>`
- `pub struct ProviderRouter`
- `impl ProviderRouter :: fn new() -> Self`
- `impl ProviderRouter :: fn register(& self, provider : Arc < dyn ModelProvider , >)`
- `impl ProviderRouter :: fn provider(& self, provider_id : & str) -> Option <Arc <dyn ModelProvider ,> ,>`
- `pub struct GeminiProvider`
- `impl GeminiProvider :: fn new(api_key : impl Into < String >) -> Self` — Varsayılan model: gemini-2.0-flash (ücretsiz)
- `impl GeminiProvider :: fn with_model(mut self, model : impl Into < String >) -> Self`
- `pub struct OpenAiProvider`
- `pub struct OllamaProvider`
- `pub struct AnthropicProvider`
- `impl AnthropicProvider :: fn new() -> Result <Self , AiError>` — Varsayılan model: claude-sonnet-4-5
- `impl AnthropicProvider :: fn with_model(mut self, model : impl Into < String >) -> Self` — Farklı bir model ile oluştur.
- `pub trait ModelProvider`
- `pub struct TokenBudget`
- `impl TokenBudget :: fn remaining(& self) -> usize`
- `pub struct StreamingSession`
- `pub struct StreamChunk`
- `pub enum AiError`
- `pub trait StructuredOutput`

### `api` (katman 0, 1731 LOC)

**Bağımlı olduğu:** agents, events, metrics, orchestration, persistence, remote, runtime, scripting, security, task, types, wasm, workflows

**Kendisine bağımlı olanlar:** tests

**Public API:**

- `pub struct HealthResponse`
- `pub fn build_router() -> Router <AppState>`
- `pub struct AgentStatus`
- `pub struct WorkflowStatus`
- `pub struct AppState`
- `pub struct RegisterNodeRequest`
- `pub struct ModuleUploadRequest`
- `pub struct ScriptRunRequest`
- `pub struct TaskSubmitRequest`
- `pub struct AgentStartRequest`
- `pub struct WorkflowSubmitRequest`
- `pub struct WorkflowStepRequest`
- `pub struct SubscriptionRequest`
- `pub async fn ws_upgrade_handler(ws : WebSocketUpgrade, State (state) : State < AppState >) -> impl IntoResponse` — WebSocket upgrade endpoint. Router'a şöyle eklenir: .route("/ws", get(ws_upgrade_handler))
- `pub struct WebsocketEvent`
- `pub struct RealtimeEvent`
- `pub async fn auth_middleware(request : Request, next : Next) -> Response` — Bearer token'ı doğrula ve Claims'i extension'a ekle. Kullanım (router.rs'de): ```rust use axum::middleware; router.route_layer(middleware::from_fn_with_state( state, auth_middleware, )) ```
- `pub async fn api_key_middleware(request : Request, next : Next) -> Response` — API key doğrulama (header: X-Api-Key).
- `pub struct ApiServer`
- `impl ApiServer :: fn new(runtime : RuntimeHandle, events : EventBus, persistence : Arc < PersistenceEngine >, metrics : Arc < RuntimeMetrics >, module_store : Arc < ModuleStore >, addr : SocketAddr) -> Self`
- `impl ApiServer :: async fn serve(self) -> Result <() , Box <dyn std::error::Error>>`
- `pub struct WorkflowRoutes`
- `impl WorkflowRoutes :: fn submit(workflow : WorkflowSubmission) -> Uuid`
- `pub struct WorkflowSubmission`
- `pub struct AgentRegistration`
- `pub struct WorkflowSubmission`
- `pub struct TaskSubmission`
- `pub struct AgentDescriptor`
- `pub async fn dashboard_handler() -> impl IntoResponse` — GET /dashboard → HTML dashboard

### `bridge` (katman 0, 776 LOC)

**Bağımlı olduğu:** events, logging, metrics, persistence, runtime, task, types, wasm

**Public API:**

- `pub fn init_app()`
- `pub async fn initialize_runtime(db_path : String, worker_count : u32) -> Result <() , String>` — AetherOS runtime'ı başlat. Flutter uygulaması açılırken (main() veya splash screen'de) bir kez çağrılır. İkinci çağrı hata döner. `db_path`: SQLite/sled veritabanı konumu Android: `/data/data/<package>/databases/aetheros.db` iOS:     `<documents>/aetheros.db` `worker_count`: paralel WASM worker sayısı Mobil için 2-4 önerilir.
- `pub fn is_runtime_ready() -> bool` — Runtime'ın çalışıp çalışmadığını kontrol et.
- `pub fn get_runtime_info() -> RuntimeInfo` — Runtime bilgilerini al (versiyon, backend, worker sayısı).
- `pub async fn submit_task(request : TaskRequest) -> Result <String , String>` — Task gönder → task_id döner. WASM modülü yoksa (wasm_module_hash == "") task yine kabul edilir; runtime Failed olarak işaretler. Örnek (Dart): ```dart final taskId = await AetherApi.submitTask(TaskRequest( wasmModuleHash: hash, entrypoint: "run", priority: "normal", timeoutMs: 30000, maxRetries: 3, )); ```
- `pub async fn get_task_status(task_id : String) -> Result <TaskStatusResponse , String>` — Task durumunu sorgula.
- `pub async fn list_tasks(limit : u32) -> Result <Vec <TaskStatusResponse> , String>` — Tüm task'ları listele (son N tane).
- `pub async fn upload_wasm_module(bytes : Vec < u8 >) -> Result <ModuleUploadResponse , String>` — WASM modülü yükle → hash döner. Flutter, dosyayı bytes olarak Rust'a verir. Rust, ModuleStore'a kaydeder ve SHA-256 hash döner. Sonraki task'larda bu hash kullanılır. Örnek (Dart): ```dart final bytes = await File("my_module.wasm").readAsBytes(); final hash = await AetherApi.uploadWasmModule(bytes: bytes); ```
- `pub fn check_module_exists(hash_hex : String) -> Result <bool , String>` — Belirtilen hash'e sahip modül runtime'da kayıtlı mı? WasmModuleScreen açılışında, eski oturumdan kalan meta-data'yı doğrulamak için her modül için çağrılır.
- `pub async fn resubmit_task(task_id : String) -> Result <String , String>` — Mevcut bir task'ı orijinal ayarlarıyla (hash, entrypoint, priority, retry policy) yeni bir UUID altında yeniden kuyruğa ekler. "Yeniden Dene" butonu için — sadece id ve zaman damgaları yenilenir, deadline sıfırlanır.
- `pub fn get_recent_logs(limit : u32) -> Result <Vec <LogRecord> , String>` — Son `limit` kadar log entry döndür (yeniden eskiye sıralı). LogScreen 2 saniyede bir bu fonksiyonu polling ile çeker. limit: 0 → varsayılan 100.
- `pub fn get_task_logs(task_id : String, limit : u32) -> Result <Vec <LogRecord> , String>` — Belirli bir task'a ait log entry'leri döndür. Task detay modalındaki "Loglar" sekmesi için. limit: 0 → varsayılan 50.
- `pub async fn get_metrics() -> Result <MetricsSnapshot , String>` — Anlık metrik görüntüsü al.
- `pub fn init_mobile_runtime(db_path : String, worker_count : usize) -> Result <() , String>` — Runtime'ı başlat. Flutter tarafından uygulama açılışında bir kez çağrılır. İkinci çağrı AlreadyInitialized hatası döner.
- `pub fn get_runtime() -> Option <& 'static MobileRuntime>` — Global runtime'ı al. init_mobile_runtime() çağrılmadan önce kullanılırsa None döner. Bridge fonksiyonları bu durumda "RuntimeNotInitialized" hatası verir.
- `pub fn block_on(fut : F) -> T` — Tokio runtime üzerinden async blok çalıştır. FRB zaten tokio context içinde çağırır; bu fonksiyon doğrudan `await` kullanamayan yerlerde (sync context) işe yarar.
- `pub struct MobileRuntime`
- `pub struct TaskRequest` — Flutter'ın task göndermek için kullandığı tip. wasm_module_hash: 64 karakter hex string (SHA-256) Boş string → modülsüz task (test için)
- `pub struct TaskStatusResponse` — Flutter'ın task durumunu göstermek için kullandığı tip.
- `pub struct RuntimeInfo`
- `pub struct ModuleUploadResponse`
- `pub struct LogRecord`

### `config` (katman 0, 8 LOC)

_Public API yok._

### `errors` (katman 0, 163 LOC)

**Kendisine bağımlı olanlar:** agents, orchestration, persistence, runtime, scripting, task, tests, wasm, worker, workflows

**Public API:**

- `pub enum PersistenceError`
- `pub enum WasmError`
- `pub enum RuntimeError`
- `pub enum TaskError`

### `events` (katman 2, 139 LOC)

**Bağımlı olduğu:** types

**Kendisine bağımlı olanlar:** api, bridge, logging, metrics, runtime, tests, worker

**Public API:**

- `pub enum TaskEvent`
- `pub enum TelemetryEvent`
- `pub enum WorkerEvent`
- `pub enum SystemEvent`
- `pub struct EventBus`
- `impl EventBus :: fn new(capacity : usize) -> Self`
- `impl EventBus :: fn publish(& self, event : SystemEvent)`
- `impl EventBus :: fn subscribe(& self) -> broadcast::Receiver <SystemEvent>`
- `pub enum RuntimeEvent`

### `logging` (katman 1, 622 LOC)

**Bağımlı olduğu:** events, orchestration, workflows

**Kendisine bağımlı olanlar:** bridge

**Public API:**

- `pub fn correlation_id() -> String`
- `pub struct AuditEvent`
- `pub struct WorkflowTrace`
- `pub trait LogExporter`
- `pub struct ConsoleExporter`
- `pub fn export(message : & str)`
- `pub struct TraceSpan`
- `pub struct DistributedTrace`
- `pub struct TelemetrySystem`
- `impl TelemetrySystem :: fn init()`
- `impl TelemetrySystem :: fn init_json()`
- `pub struct MetricsAggregator`
- `impl MetricsAggregator :: fn new() -> Self`
- `impl MetricsAggregator :: fn increment_total(& self)`
- `impl MetricsAggregator :: fn increment_failed(& self)`
- `pub struct LineageEvent`
- `pub struct RuntimeTrace`
- `pub struct RuntimeMetrics`
- `impl RuntimeMetrics :: fn new() -> Self`
- `impl RuntimeMetrics :: fn increment_tasks(& self)`
- `impl RuntimeMetrics :: fn completed_tasks(& self) -> u64`
- `pub struct AiInferenceTrace`
- `pub async fn log_collector(bus : EventBus, buffer : LogBuffer)`
- `pub struct LogBuffer`
- `impl LogBuffer :: fn new() -> Self`
- `impl LogBuffer :: fn clear(& self)`
- `impl LogBuffer :: fn len(& self) -> usize`

### `metrics` (katman 1, 236 LOC)

**Bağımlı olduğu:** events

**Kendisine bağımlı olanlar:** api, bridge, tests

**Public API:**

- `pub struct TaskMetrics`
- `impl TaskMetrics :: fn new() -> Self`
- `impl TaskMetrics :: fn increment_completed(& self)`
- `impl TaskMetrics :: fn increment_failed(& self)`
- `impl TaskMetrics :: fn increment_cancelled(& self)`
- `impl TaskMetrics :: fn increment_retries(& self)`
- `pub struct WorkerMetrics`
- `pub struct MetricsSnapshot`
- `pub struct RuntimeMetrics`
- `impl RuntimeMetrics :: fn new() -> Self`
- `impl RuntimeMetrics :: fn increment_completed(& self)`
- `impl RuntimeMetrics :: fn increment_failed(& self)`
- `impl RuntimeMetrics :: fn increment_queued(& self)`
- `impl RuntimeMetrics :: fn increment_retried(& self)`
- `impl RuntimeMetrics :: fn set_active_workers(& self, count : u64)`
- `impl RuntimeMetrics :: fn snapshot(& self) -> MetricsSnapshot`
- `impl RuntimeMetrics :: fn start_collecting(self : Arc < Self >, bus : EventBus)` — EventBus'tan beslenme döngüsünü arka planda başlat. Her SystemEvent::Task event'ine göre metrikleri günceller. Shutdown: bus kapanınca (RecvError::Closed) döngü çıkar.
- `impl RuntimeMetrics :: fn process_event(& self, event : & SystemEvent)`

### `orchestration` (katman 2, 1933 LOC)

**Bağımlı olduğu:** errors, remote, runtime, task, types, workflows

**Kendisine bağımlı olanlar:** api, logging, tests, workflows

**Public API:**

- `pub struct ExecutionScheduler`
- `impl ExecutionScheduler :: fn ready_nodes(graph : & ExecutionGraph, completed : & std :: collections :: HashSet < Uuid , >) -> Vec <Uuid>`
- `impl ExecutionScheduler :: fn topological_sort(graph : & ExecutionGraph) -> Vec <Uuid>`
- `pub struct GraphExecutor`
- `impl GraphExecutor :: async fn execute(graph : ExecutionGraph) -> Result <() , RuntimeError>`
- `pub struct DurableEventStream`
- `impl DurableEventStream :: fn new() -> Self`
- `impl DurableEventStream :: fn publish(& self, event : OrchestrationEvent)`
- `impl DurableEventStream :: fn subscribe(& self) -> broadcast::Receiver <OrchestrationEvent ,>`
- `pub struct ReplaySession`
- `pub struct ReplayState`
- `pub struct ReplayEngine`
- `impl ReplaySession :: fn new(execution_id : Uuid) -> Self`
- `impl ReplaySession :: fn append(& mut self, event : OrchestrationEvent)`
- `impl ReplaySession :: fn event_count(& self) -> usize`
- `impl ReplayEngine :: fn remaining_nodes(graph : & ExecutionGraph, replay : & ReplayState) -> Vec <ExecutionNode>`
- `pub struct ExecutionMetrics`
- `impl ExecutionMetrics :: fn new() -> Self`
- `impl ExecutionMetrics :: fn increment_active(& self)`
- `impl ExecutionMetrics :: fn increment_completed(& self)`
- `impl ExecutionMetrics :: fn increment_failed(& self)`
- `pub enum DurabilityLevel`
- `pub enum ConsensusState`
- `pub struct RuntimeSupervisor`
- `impl RuntimeSupervisor :: fn new() -> Self`
- `impl RuntimeSupervisor :: fn register(& mut self, id : Uuid)`
- `impl RuntimeSupervisor :: fn update(& mut self, id : Uuid, state : OrchestrationState)`
- `impl RuntimeSupervisor :: fn state(& self, id : & Uuid) -> Option <OrchestrationState ,>`
- `pub struct ExecutionOwnership`
- `pub struct QuorumState`
- `pub struct QuorumPolicy`
- `impl QuorumState :: fn has_quorum(& self) -> bool`
- `impl QuorumPolicy :: fn satisfied(& self, available : usize) -> bool`
- `pub trait NodeDispatcher`
- `pub struct ExecutionDispatcher<T>`
- `impl ExecutionDispatcher <T> :: fn new(transport : Arc < T >, cluster : Arc < ClusterState >, runtime : RuntimeHandle) -> Self`
- `pub struct ExecutionLineage`
- `pub struct StateConvergence`
- `impl StateConvergence :: fn new() -> Self`
- `impl StateConvergence :: fn update(& mut self, execution_id : Uuid, state : OrchestrationState)`
- `impl StateConvergence :: fn converged(& self, execution_id : & Uuid) -> bool`
- `pub struct StateTransition`
- `pub struct OrchestrationStateMachine`
- `impl OrchestrationStateMachine :: fn can_transition(from : OrchestrationState, to : OrchestrationState) -> bool`
- `pub fn execution_checksum(payload : & [u8]) -> String`
- `pub struct ExecutionSnapshot`
- `pub enum FailureSeverity`
- `pub struct FailureClassification`
- `pub struct OrchestrationEventBus`
- `impl OrchestrationEventBus :: fn new() -> Self`
- `impl OrchestrationEventBus :: fn publish(& self, event : OrchestrationEvent)`
- `impl OrchestrationEventBus :: fn subscribe(& self) -> broadcast::Receiver <OrchestrationEvent ,>`
- `pub struct RecoveryPlan`
- `pub struct SupervisionTree`
- `impl SupervisionTree :: fn new() -> Self`
- `impl SupervisionTree :: fn attach(& mut self, child : Uuid, parent : Uuid)`
- `impl SupervisionTree :: fn parent(& self, child : & Uuid) -> Option <& Uuid>`
- `pub struct ClusterTopology`
- `pub struct FenceToken`
- `pub struct CoordinationLease`
- `pub struct ExecutionCoordinator`
- `impl CoordinationLease :: fn expired(& self) -> bool`
- `impl ExecutionCoordinator :: fn new() -> Self`
- `impl ExecutionCoordinator :: fn ready_nodes(& self, graph : & ExecutionGraph) -> Vec <Uuid>` — Bağımlılıkları karşılanmış, çalışmaya hazır node'ları döndür.
- `impl ExecutionCoordinator :: fn mark_completed(& mut self, id : Uuid)`
- `impl ExecutionCoordinator :: fn is_complete(& self, graph : & ExecutionGraph) -> bool`
- `pub trait GraphPersistence`
- `pub struct BackpressurePolicy`
- `pub struct JournalEntry`
- `pub struct ExecutionLease`
- `impl ExecutionLease :: fn renew(& mut self, duration : Duration)`
- `impl ExecutionLease :: fn expired(& self) -> bool`
- `pub struct ReconciliationResult`
- `pub struct ReconciliationEngine`
- `impl ReconciliationEngine :: fn reconcile(execution_id : Uuid) -> ReconciliationResult`
- `pub struct ArbitrationDecision`
- `pub struct ArbitrationEngine`
- `impl ArbitrationEngine :: fn decide(execution_id : Uuid, candidates : Vec < Uuid >) -> Option <ArbitrationDecision ,>`
- `pub struct ElectionResult`
- `pub struct LeaderElection`
- `impl LeaderElection :: fn elect(cluster : & ClusterState) -> Option <ElectionResult>` — Sağlıklı node'lar arasından leader seç. Algoritma: 1. Quorum yok → None 2. Sağlıklı node'ları al 3. Her node için load_score hesapla 4. En düşük yüklü = aday 5. Tie: UUID lexicographic (deterministic) Gerçek dağıtık sistemde bu kısmi — tam Raft/Paxos her node'un oy gondermesi gerektirir. Bu implementasyon coordinator tarafından çalıştırılan merkezi versiyondur.
- `impl LeaderElection :: fn elect_simple(nodes : Vec < Uuid >) -> Option <Uuid>` — Eski API — geriye uyumluluk. Sadece winner UUID'i döndürür.
- `pub struct ExecutionGraph`
- `pub struct ExecutionNode`
- `pub struct ExecutionEdge`
- `pub enum ExecutionNodeKind`
- `pub struct ExecutionMetadata`
- `pub struct FailoverDecision`
- `pub struct FailoverEngine`
- `impl FailoverEngine :: fn should_failover(healthy : bool) -> bool`
- `pub struct ExecutionContext`
- `pub struct ExecutionResult<T>`
- `pub trait ExecutionUnit`
- `pub struct DistributedCheckpoint`
- `pub struct OrchestrationEvent`
- `pub enum OrchestrationState`
- `pub struct OwnershipTransfer`
- `pub struct ResiliencePolicy`
- `pub struct RetryPolicy`
- `pub struct ExecutionPolicy`
- `pub struct RuntimeSnapshot`

### `persistence` (katman 1, 334 LOC)

**Bağımlı olduğu:** errors, task, types

**Kendisine bağımlı olanlar:** api, bridge, runtime, tests, worker

**Public API:**

- `pub struct RecoveryEngine`
- `impl RecoveryEngine :: fn recoverable_tasks(tasks : Vec < PersistedTask >) -> Vec <PersistedTask>`
- `impl RecoveryEngine :: fn mark_recovered(task : & mut PersistedTask)`
- `pub struct PersistenceEngine`
- `impl PersistenceEngine :: fn open(path : & str) -> Result <Self , PersistenceError>`
- `impl PersistenceEngine :: fn persist_task(& self, task : & PersistedTask) -> Result <() , PersistenceError>` — Task'ı MessagePack formatında kaydet. rmp_serde::to_vec_named → field isimlerini korur (schema evrimi için önemli).
- `impl PersistenceEngine :: fn update_task_state(& self, task_id : & TaskId, new_state : TaskState) -> Result <() , PersistenceError>`
- `impl PersistenceEngine :: fn set_task_failed(& self, task_id : & TaskId, error : String) -> Result <() , PersistenceError>` — Task'ı Failed durumuna geçir VE gerçek hata mesajını kaydet. Worker, WasmError::to_string() çıktısını buraya geçirir (örn. "missing entrypoint", "invalid module: module not found in store"). Bridge katmanı bu mesajı doğrudan TaskStatusResponse.error_message'a yansıtır — böylece ADB olmadan gerçek hata UI'da görülebilir.
- `impl PersistenceEngine :: fn load_task(& self, task_id : & TaskId) -> Result <Option <PersistedTask> , PersistenceError>`
- `impl PersistenceEngine :: fn delete_task(& self, task_id : & TaskId) -> Result <() , PersistenceError>`
- `impl PersistenceEngine :: fn load_all_tasks(& self) -> Result <Vec <PersistedTask> , PersistenceError>`
- `impl PersistenceEngine :: async fn flush_async(& self) -> Result <() , PersistenceError>` — Async flush — executor thread'ini bloklamaz.
- `impl PersistenceEngine :: fn start_background_flush(self : Arc < Self >, period_ms : u64, mut shutdown_rx : tokio :: sync :: watch :: Receiver < bool >)` — Arka plan flush — 100ms aralıkla.
- `impl PersistenceEngine :: fn database(& self) -> & Db`
- `impl PersistenceEngine :: fn snapshots(& self) -> & Tree`
- `pub struct RuntimeSnapshot`
- `pub struct PersistedTask`
- `impl PersistedTask :: fn is_terminal(& self) -> bool`

### `plugins` (katman 0, 206 LOC)

**Public API:**

- `pub enum PluginState`
- `pub struct SandboxPolicy`
- `pub enum PluginCapability`
- `pub struct PluginCapabilities`
- `pub trait Plugin`
- `pub struct PluginVersion`
- `pub trait PluginHooks`
- `pub struct PluginRegistry`
- `impl PluginRegistry :: fn new() -> Self`
- `impl PluginRegistry :: fn register(& self, plugin : Arc < dyn Plugin >)`
- `impl PluginRegistry :: fn get(& self, name : & str) -> Option <Arc <dyn Plugin> ,>`
- `pub struct IsolationBoundary`

### `registry` (katman 0, 178 LOC)

**Bağımlı olduğu:** task, types, worker

**Public API:**

- `pub struct TaskRegistry`
- `impl TaskRegistry :: fn new() -> Self`
- `impl TaskRegistry :: fn insert(& self, task : TaskDefinition)`
- `pub struct WorkerRegistry`
- `impl WorkerRegistry :: fn new() -> Self`
- `impl WorkerRegistry :: fn register(& self, id : WorkerId, state : WorkerState)`
- `impl WorkerRegistry :: fn update_state(& self, id : & WorkerId, state : WorkerState)`
- `impl WorkerRegistry :: fn remove(& self, id : & WorkerId)`
- `impl WorkerRegistry :: fn list(& self) -> Vec <(WorkerId , WorkerState)>`
- `impl WorkerRegistry :: fn count_by_state(& self, state : WorkerState) -> usize`
- `impl WorkerRegistry :: fn total(& self) -> usize`
- `pub struct RegistryEntry`
- `pub struct RuntimeRegistry`
- `impl RuntimeRegistry :: fn new() -> Self`
- `impl RuntimeRegistry :: fn register(& self, info : RegistryEntry)`
- `impl RuntimeRegistry :: fn get(& self, id : & RuntimeId) -> Option <RegistryEntry>`
- `impl RuntimeRegistry :: fn remove(& self, id : & RuntimeId)`
- `impl RuntimeRegistry :: fn list(& self) -> Vec <RegistryEntry>`
- `impl RuntimeRegistry :: fn count(& self) -> usize`

### `remote` (katman 0, 981 LOC)

**Kendisine bağımlı olanlar:** api, orchestration, tests

**Public API:**

- `pub enum RemoteCommand`
- `pub struct DiscoveryRecord`
- `pub struct JoinRequest` — Basit HTTP-tabanlı join payload.
- `pub struct DiscoveryService`
- `pub struct StaticDiscovery` — Basit static discovery — env/config dosyasından seed'ler.
- `impl DiscoveryService :: fn new(self_id : Uuid, self_address : impl Into < String >, seeds : Vec < String >, cluster : Arc < ClusterState >) -> Self`
- `impl DiscoveryService :: async fn join(& self)` — Cluster'a katıl: her seed node'a join isteği gönder.
- `impl DiscoveryService :: async fn leave(& self)` — Cluster'dan temiz ayrıl.
- `impl DiscoveryService :: async fn probe(& self, address : & str) -> Result <bool , String>` — Node canlı mı? GET /health endpoint'ini kontrol et.
- `impl DiscoveryService :: fn start(self : Arc < Self >, interval_secs : u64, mut shutdown_rx : tokio :: sync :: watch :: Receiver < bool >)` — Periyodik discovery döngüsü. Her `interval` saniyede bir seed'leri kontrol et.
- `impl StaticDiscovery :: fn from_env() -> Vec <String>` — AETHEROS_SEEDS="addr1:9000,addr2:9000,addr3:9000"
- `pub enum NodeHealth`
- `pub enum NodeCapability`
- `pub struct RemoteNode`
- `impl RemoteNode :: fn new(address : impl Into < String >, capabilities : Vec < NodeCapability >) -> Self`
- `impl RemoteNode :: fn touch(& mut self)`
- `impl RemoteNode :: fn mark_unhealthy(& mut self)`
- `pub struct ClusterState`
- `impl ClusterState :: fn new() -> Self`
- `impl ClusterState :: fn register(& self, node : RemoteNode)`
- `impl ClusterState :: fn remove(& self, node_id : & Uuid)`
- `impl ClusterState :: fn update_heartbeat(& self, hb : Heartbeat)`
- `impl ClusterState :: fn nodes(& self) -> Vec <RemoteNode>`
- `impl ClusterState :: fn healthy_nodes(& self) -> Vec <RemoteNode>`
- `impl ClusterState :: fn nodes_with_capability(& self, cap : & NodeCapability) -> Vec <RemoteNode>`
- `impl ClusterState :: fn node_ids(& self) -> HashSet <Uuid>`
- `impl ClusterState :: fn get_node(& self, id : & Uuid) -> Option <RemoteNode>`
- `impl ClusterState :: fn heartbeat(& self, id : & Uuid) -> Option <Heartbeat>`
- `impl ClusterState :: fn all_heartbeats(& self) -> Vec <Heartbeat>`
- `impl ClusterState :: fn evict_stale_nodes(& self)` — Stale node'ları unhealthy olarak işaretle. Periyodik olarak çağrılmalı (health check loop).
- `impl ClusterState :: fn health(& self) -> ClusterHealth`
- `impl ClusterState :: fn has_quorum(& self) -> bool` — n/2 + 1 quorum var mı?
- `impl ClusterState :: fn quorum_size(& self) -> usize`
- `impl ClusterState :: fn set_leader(& self, node_id : Uuid)`
- `impl ClusterState :: fn leader(& self) -> Option <Uuid>`
- `impl ClusterState :: fn is_leader(& self, node_id : & Uuid) -> bool`
- `impl ClusterState :: fn size(& self) -> usize`
- `impl ClusterState :: fn healthy_count(& self) -> usize`
- `pub trait ClusterReplication`
- `pub struct TransportSecurity`
- `pub struct Heartbeat`
- `pub struct NodeSelection` — Node seçim sonucu.
- `pub struct RemoteScheduler`
- `impl RemoteScheduler :: fn select_node(nodes : & [RemoteNode], required_capability : & NodeCapability, heartbeats : & [Heartbeat]) -> Option <NodeSelection>` — Capability'e göre uygun, en az yüklü node'u seç. Algoritma: 1. healthy == true filtrele 2. last_seen < 30s (stale node'ları ele) 3. İstenen capability'e sahip node'ları filtrele 4. Heartbeat'e göre load_score hesapla 5. En düşük score'lu node'u seç
- `impl RemoteScheduler :: fn select_any(nodes : & [RemoteNode]) -> Option <Uuid>` — Basit seçim (geriye uyumluluk).
- `pub enum ClusterHealth`
- `pub trait RemoteTransport`
- `pub struct HttpTransport`
- `impl HttpTransport :: fn new(cluster : Arc < ClusterState >) -> Self`
- `pub struct TaskLease`

### `runtime` (katman 1, 759 LOC)

**Bağımlı olduğu:** errors, events, persistence, task, wasm, worker

**Kendisine bağımlı olanlar:** api, bridge, orchestration, tests, workflows

**Public API:**

- `pub struct Scheduler`
- `impl Scheduler :: fn new(queue : Arc < PriorityTaskQueue >) -> Self`
- `impl Scheduler :: async fn submit(& self, task : TaskDefinition)`
- `pub enum RuntimeState`
- `impl RuntimeState :: fn from_u8(value : u8) -> Self` — AtomicU8 değerini enum'a dönüştür. Bilinmeyen bir değer gelirse `Failed` döner — panic etmez, observable bir hata state'i üretir.
- `impl RuntimeState :: fn is_terminal(self) -> bool` — State'in terminal (nihai) olup olmadığı.
- `impl RuntimeState :: fn is_running(self) -> bool` — Dışarıya "hazır" sinyali verilip verilmeyeceği.
- `pub struct RuntimeHandle`
- `impl RuntimeHandle :: fn new(sender : mpsc :: Sender < TaskDefinition >) -> Self`
- `impl RuntimeHandle :: async fn submit(& self, task : TaskDefinition) -> Result <() , RuntimeError>`
- `pub struct RuntimeConfig`
- `pub trait RuntimeService`
- `pub struct Dispatcher`
- `impl Dispatcher :: fn new(queue : Arc < PriorityTaskQueue >, workers : Arc < RwLock < WorkerManager > >, backpressure : BackpressureController) -> Self`
- `impl Dispatcher :: async fn run(& self) -> Result <() , RuntimeError>`
- `impl Dispatcher :: async fn shutdown_workers(& self)` — Faz 1'den: Tüm worker'lara Shutdown sinyali gönder.
- `pub struct ShutdownController`
- `impl ShutdownController :: fn new() -> Self`
- `impl ShutdownController :: fn cancel(& self)`
- `impl ShutdownController :: async fn wait(& self)`
- `impl ShutdownController :: async fn wait_timeout(& self, duration : std :: time :: Duration) -> bool`
- `impl ShutdownController :: fn child_token(& self) -> CancellationToken`
- `pub struct RuntimeBootstrap`
- `impl RuntimeBootstrap :: fn build(config : RuntimeConfig) -> Result <Self , RuntimeError>`
- `impl RuntimeBootstrap :: fn runtime_handle(& self) -> RuntimeHandle` — API katmanı için RuntimeHandle. RuntimeHandle, mpsc::Sender'ı sarar. Clone'lanabilir — ApiServer, Agent, Workflow hepsi aynı handle'ı kullanabilir.
- `impl RuntimeBootstrap :: fn runtime(self) -> Runtime`
- `impl RuntimeBootstrap :: fn sender(& self) -> mpsc::Sender <TaskDefinition>` — Ham sender — geriye dönük uyumluluk için.
- `pub struct Runtime`
- `impl Runtime :: fn new(config : RuntimeConfig, task_receiver : mpsc :: Receiver < TaskDefinition >) -> Result <Self , RuntimeError>`
- `impl Runtime :: async fn start(mut self) -> Result <() , RuntimeError>`
- `impl Runtime :: fn shutdown(& self)` — Runtime'ı dışarıdan durdur (örn. sinyal handler'dan).
- `impl Runtime :: fn events(& self) -> EventBus`
- `impl Runtime :: fn config(& self) -> & RuntimeConfig`
- `impl Runtime :: fn persistence(& self) -> Arc <PersistenceEngine>`
- `impl Runtime :: fn module_store(& self) -> Arc <ModuleStore>` — WASM modül deposu — upload edilen binary'lerin hash → bytes eşlemesi. Bridge katmanı (upload_wasm_module) bu handle üzerinden yeni modülleri kaydeder; WasmiEngine (worker'lar içinde) aynı Arc'ı paylaşır, böylece upload edilen modül execute sırasında bulunabilir.
- `impl Runtime :: fn state(& self) -> RuntimeState`
- `pub struct BackpressureController`
- `impl BackpressureController :: fn new(max_inflight : usize) -> Self`
- `impl BackpressureController :: async fn acquire(& self) -> tokio::sync::OwnedSemaphorePermit`
- `impl BackpressureController :: fn available(& self) -> usize`

### `scripting` (katman 0, 403 LOC)

**Bağımlı olduğu:** agents, errors, task, types, wasm

**Kendisine bağımlı olanlar:** api, tests

**Public API:**

- `pub struct ScriptResult`
- `impl ScriptResult :: fn output_as_string(& self) -> String` — Çıktıyı UTF-8 string olarak al. Geçersiz byte'lar lossy replace ile işlenir.
- `impl ScriptResult :: fn is_empty(& self) -> bool`
- `pub struct ScriptTool`
- `impl ScriptTool :: fn new(script : ScriptDefinition, engine : Arc < ScriptEngine >) -> Self`
- `pub struct ScriptDefinition`
- `impl ScriptDefinition :: fn from_wat(name : impl Into < String >, wat_source : & str, entrypoint : impl Into < String >, timeout_ms : u64) -> Result <Self , WasmError>` — WAT (WebAssembly Text Format) stringinden script oluştur. Kullanıcı doğrudan metin script yazabilir: ```wat (module (func (export "main")) ) ```
- `impl ScriptDefinition :: fn from_binary(name : impl Into < String >, wasm_binary : Vec < u8 >, entrypoint : impl Into < String >, timeout_ms : u64) -> Self` — Derlenmiş WASM binary'den script oluştur.
- `impl ScriptDefinition :: fn from_hex(name : impl Into < String >, hex_str : & str, entrypoint : impl Into < String >, timeout_ms : u64) -> Result <Self , WasmError>` — Hex string'den binary'ye çevirerek oluştur. REST API üzerinden hex olarak gelen script'ler için.
- `impl ScriptDefinition :: fn with_description(mut self, desc : impl Into < String >) -> Self`
- `pub struct ScriptRegistry`
- `impl ScriptRegistry :: fn new() -> Self`
- `impl ScriptRegistry :: fn register(& self, script : ScriptDefinition)` — Script'i ismiyle kaydet.
- `impl ScriptRegistry :: fn get(& self, name : & str) -> Option <ScriptDefinition>` — İsimle script'i bul.
- `impl ScriptRegistry :: fn remove(& self, name : & str)`
- `impl ScriptRegistry :: fn list(& self) -> Vec <ScriptDefinition>`
- `impl ScriptRegistry :: fn count(& self) -> usize`
- `impl ScriptRegistry :: fn contains(& self, name : & str) -> bool`
- `pub struct ScriptEngine`
- `impl ScriptEngine :: fn new(wasm : Arc < dyn WasmExecutor >) -> Self`
- `impl ScriptEngine :: async fn run(& self, script : & ScriptDefinition) -> Result <ScriptResult , WasmError>` — Script'i çalıştır → ScriptResult döndür. Execution süresi ölçülür. WasmEngine hatası → WasmError olarak iletilir.

### `sdk` (katman 0, 240 LOC)

**Public API:**

- `pub struct PythonSdk`
- `pub enum ClientError`
- `pub struct SubmitTaskRequest`
- `pub struct StartAgentRequest`
- `pub struct SubmitWorkflowRequest`
- `pub struct SubmitTaskResponse`
- `pub struct TaskStateResponse`
- `pub struct AgentResponse`
- `pub struct WorkflowResponse`
- `pub struct HealthResponse`
- `pub struct AetherClient`
- `impl AetherClient :: fn new(endpoint : impl Into < String >) -> Self`
- `impl AetherClient :: async fn submit_task(& self, req : SubmitTaskRequest) -> Result <SubmitTaskResponse , ClientError>` — POST /tasks → task submit
- `impl AetherClient :: async fn task_state(& self, task_id : Uuid) -> Result <TaskStateResponse , ClientError>` — GET /tasks/:id → task state
- `impl AetherClient :: async fn cancel_task(& self, task_id : Uuid) -> Result <serde_json::Value , ClientError>` — POST /tasks/:id/cancel
- `impl AetherClient :: async fn start_agent(& self, objective : impl Into < String >, max_steps : usize, max_tokens : usize) -> Result <AgentResponse , ClientError>` — POST /agents → agent başlat
- `impl AetherClient :: async fn submit_workflow(& self, name : impl Into < String >) -> Result <WorkflowResponse , ClientError>` — POST /workflows → workflow submit
- `impl AetherClient :: async fn health(& self) -> Result <HealthResponse , ClientError>` — GET /health
- `pub struct TypeScriptSdk`

### `security` (katman 0, 670 LOC)

**Kendisine bağımlı olanlar:** api, tests

**Public API:**

- `pub struct AuthToken`
- `pub struct Claims`
- `pub struct ApiKey`
- `pub struct TokenManager`
- `pub enum TokenError`
- `impl Claims :: fn new(subject : impl Into < String >, role : Role, ttl_seconds : u64) -> Self`
- `impl Claims :: fn is_expired(& self) -> bool`
- `impl Claims :: fn ttl_remaining(& self) -> u64`
- `impl ApiKey :: fn new(raw : impl Into < String >) -> Self`
- `impl ApiKey :: fn verify(& self, provided : & str) -> bool` — Verilen ham key'in hash'i kaydedilen hash ile eşleşiyor mu?
- `impl ApiKey :: fn raw(& self) -> & str` — Ham API key'i döndür (log maskeleme veya audit için).
- `impl ApiKey :: fn hash(& self) -> & str`
- `impl TokenManager :: fn new(secret : impl Into < String >, default_ttl_seconds : u64) -> Self`
- `impl TokenManager :: fn from_env() -> Self` — Ortam değişkeninden oluştur.
- `impl TokenManager :: fn generate(& self, subject : impl Into < String >, role : Role) -> AuthToken` — Claims → imzalı JWT token üret. Basit implementasyon: header.payload.signature Base64url encode + HMAC-SHA256 imza. Üretimde: jsonwebtoken crate ile değiştir.
- `impl TokenManager :: fn verify(& self, token : & str) -> Result <Claims , TokenError>` — Token → Claims (doğrulama dahil).
- `pub enum Action`
- `pub enum Role`
- `pub enum AuthzError`
- `pub struct RbacGuard`
- `impl Role :: fn can(& self, action : & Action) -> bool` — Bu role bu action'ı yapabilir mi?
- `impl Role :: fn level(& self) -> u8` — Role hiyerarşi seviyesi (yüksek = daha yetkili).
- `impl Role :: fn is_admin(& self) -> bool`
- `impl RbacGuard :: fn authorize(role : & Role, action : & Action) -> Result <() , AuthzError>` — Tek kontrol noktası.
- `pub struct CapabilitySet`
- `pub struct SecurityGovernor`
- `impl SecurityGovernor :: fn allowed(policy : & SecurityPolicy) -> bool`
- `pub trait SecretProvider`
- `pub struct AuthorizationPolicy`
- `impl AuthorizationPolicy :: fn allowed(role : & Role) -> bool`
- `pub struct AuditLog`
- `pub struct SecurityPolicy`
- `pub struct IsolationPolicy`
- `pub struct AuthenticationToken`
- `pub struct ExecutionIdentity`
- `pub struct SecurityPolicy`
- `pub trait SignatureVerifier`
- `pub struct WasmSignature`

### `task` (katman 1, 771 LOC)

**Bağımlı olduğu:** errors, tests, types, wasm

**Kendisine bağımlı olanlar:** api, bridge, orchestration, persistence, registry, runtime, scripting, tests, wasm, worker, workflows

**Public API:**

- `pub trait TaskQueue`
- `pub struct TaskContext`
- `pub struct RetryPolicy`
- `pub enum RetryClassification`
- `impl RetryPolicy :: fn should_retry(& self, attempts : u32, error : & WasmError) -> bool` — Bu hata + mevcut attempt sayısıyla retry yapılmalı mı? `attempts`: şimdiye kadar yapılan deneme sayısı. İlk denemeden sonra gelen ilk hata → attempts = 1. max_attempts = 3 ise 3 kez denenip 3. başarısızsa retry yok.
- `impl RetryPolicy :: fn classify_wasm_error(error : & WasmError) -> RetryClassification`
- `impl RetryPolicy :: fn next_delay(& self, attempt : u32) -> Duration` — Exponential backoff hesapla (jitter opsiyonel). `attempt`: kaçıncı denemeden sonra bekleniyor? 1. başarısızlık → attempt = 1 → base * 2^1 2. başarısızlık → attempt = 2 → base * 2^2 Overflow koruması: pow(attempt.min(16))
- `pub struct ExecutionLease`
- `impl ExecutionLease :: fn expired(& self) -> bool`
- `impl ExecutionLease :: fn new(task_id : TaskId, worker_id : WorkerId, ttl_seconds : i64) -> Self`
- `pub enum TaskResult`
- `pub enum TaskFailure`
- `pub enum TaskPriority`
- `impl TaskPriority :: fn weight(self) -> u8` — Öncelik ağırlığı. Yüksek değer → kuyrukta öne geçer. Bu değerler Ord implementasyonunun tek kaynağıdır. Sıra değiştirmek gerekirse sadece burası güncellenir.
- `pub struct PriorityTaskQueue`
- `impl PriorityTaskQueue :: fn new() -> Self`
- `impl PriorityTaskQueue :: async fn push(& self, task : TaskDefinition) -> Result <() , TaskError>` — Task'ı önceliğine göre doğru tier'a ekle. O(1): lock + VecDeque::push_back Sadece ilgili tier kilitlenir — diğer tier'lar serbest.
- `impl PriorityTaskQueue :: async fn pop(& self) -> Result <TaskDefinition , TaskError>` — En yüksek öncelikli tier'dan task al. O(1): tier sırası sabit (4 kontrol) + pop_front Sıralama: Critical → High → Normal → Low Boşsa: Notify bekle (spin yok)
- `impl PriorityTaskQueue :: async fn len(& self) -> usize`
- `impl PriorityTaskQueue :: async fn is_empty(& self) -> bool`
- `impl PriorityTaskQueue :: async fn distribution(& self) -> [usize ; 4]` — Tier bazlı dağılım — monitoring için.
- `pub struct TaskOutput`
- `pub fn deadline_expired(task : & TaskDefinition) -> bool`
- `pub struct TaskMetadata`
- `pub enum TaskState`
- `pub struct TaskDefinition`
- `impl TaskDefinition :: fn has_module(& self) -> bool` — Hash sıfır mı? (modül yok)
- `pub struct TaskDependency`
- `pub struct TaskOrchestration`

### `tests` (katman 0, 3231 LOC)

**Bağımlı olduğu:** api, errors, events, metrics, orchestration, persistence, remote, runtime, scripting, security, task, types, wasm, workflows

**Kendisine bağımlı olanlar:** task

**Public API:**

- `pub fn make_task(priority : TaskPriority, entrypoint : & str) -> TaskDefinition`
- `pub fn make_task_with_hash(priority : TaskPriority, hash : [u8 ; 32]) -> TaskDefinition`

### `types` (katman 1, 99 LOC)

**Kendisine bağımlı olanlar:** agents, ai, api, bridge, events, orchestration, persistence, registry, scripting, task, tests, worker, workflows

**Public API:**

- `pub struct RuntimeId`
- `pub struct WorkerId`
- `pub struct TaskId`
- `pub struct ExecutionId`
- `pub struct Versioned<T>`
- `pub struct AgentPlan` — `agents` ve `ai` modüllerinin her ikisi de bu tipe ihtiyaç duyduğu için döngüsel bağımlılığı önlemek amacıyla buraya (types) taşındı. Önceki konum: src/agents/plans.rs
- `pub struct AgentPlanStep`
- `pub trait AgentTool` — `agents` ve `ai` modüllerinin her ikisi de bu trait'e ihtiyaç duyduğu için döngüsel bağımlılığı önlemek amacıyla buraya (types) taşındı. Önceki konum: src/agents/tools.rs

### `wasm` (katman 1, 896 LOC)

**Bağımlı olduğu:** errors, task

**Kendisine bağımlı olanlar:** api, bridge, runtime, scripting, task, tests, worker

**Public API:**

- `pub enum ModuleStoreError`
- `pub struct ModuleStore`
- `impl ModuleStore :: fn new() -> Self`
- `impl ModuleStore :: fn store(& self, binary : Vec < u8 >) -> Result <ModuleHash , ModuleStoreError>` — Binary'yi depola → hash döndür. Aynı binary tekrar gönderilirse hash döner, ikinci kez depolanmaz (idempotent).
- `impl ModuleStore :: fn get(& self, hash : & ModuleHash) -> Result <Arc <Vec <u8>> , ModuleStoreError>` — Hash'e göre binary'yi al.
- `impl ModuleStore :: fn contains(& self, hash : & ModuleHash) -> bool` — Binary mevcutsa true.
- `impl ModuleStore :: fn count(& self) -> usize` — Depolanan modül sayısı.
- `impl ModuleStore :: fn list_hashes(& self) -> Vec <String>` — Tüm hash'leri hex string listesi olarak döndür. GET /modules endpoint'i için.
- `impl ModuleStore :: fn hash_to_hex(hash : & ModuleHash) -> String` — Hash'i hex string'e çevir (API response için).
- `impl ModuleStore :: fn hex_to_hash(hex_str : & str) -> Result <ModuleHash , ModuleStoreError>` — Hex string'i hash'e çevir (API request için).
- `impl ModuleStore :: fn binary_size(& self, hash : & ModuleHash) -> Option <usize>` — Belirli bir hash için binary boyutunu döndür.
- `impl ModuleStore :: fn clear(& self)` — Kullanılmayan modülleri temizle (Faz 9: LRU eviction). Şimdilik tümünü temizler — monitoring endpoint için.
- `pub trait WasmExecutor` — Ortak WASM yürütücü kontratı. Her iki backend (wasmtime / wasmi) bu trait'i implement eder. Worker, scripting ve runtime katmanları `Arc<dyn WasmExecutor>` tutar; hangi backend'in derlendiğini bilmek zorunda değildir.
- `pub fn register_host_functions(linker : & mut Linker < HostContext >) -> Result <() , wasmtime::Error>`
- `pub struct HostContext`
- `pub struct WasmExecutionState`
- `pub struct WasmiHostContext`
- `pub struct WasmiSandboxLimits`
- `pub struct WasmiEngine`
- `impl WasmiEngine :: fn new(limits : WasmiSandboxLimits, module_store : Arc < ModuleStore >) -> Self`
- `impl WasmiEngine :: fn module_store(& self) -> Arc <ModuleStore>`
- `pub struct MemoryLimiter`
- `impl MemoryLimiter :: fn new(max_memory_size : usize) -> Self`
- `impl MemoryLimiter :: fn limits(self) -> StoreLimits`
- `pub fn create_engine() -> Result <Engine , wasmtime::Error>`
- `pub struct SandboxLimits`
- `pub struct WasmEngine`
- `impl WasmEngine :: fn new(limits : SandboxLimits, module_store : Arc < ModuleStore >) -> Result <Self , WasmError>`
- `impl WasmEngine :: async fn execute(& self, task : TaskDefinition) -> Result <Vec <u8> , WasmError>`
- `impl WasmEngine :: fn cached_module_count(& self) -> usize`
- `impl WasmEngine :: fn module_store(& self) -> Arc <ModuleStore>`
- `pub struct CapabilitySet`
- `pub fn create_linker(engine : & wasmtime :: Engine) -> Result <Linker <HostContext> , wasmtime::Error ,>`
- `pub fn register_host_functions(linker : & mut Linker < WasmiHostContext >) -> Result <() , WasmError>`
- `pub struct WasmiHostContext`
- `impl WasmiHostContext :: fn new(fuel_limit : u64) -> Self`

### `worker` (katman 1, 773 LOC)

**Bağımlı olduğu:** errors, events, persistence, task, types, wasm

**Kendisine bağımlı olanlar:** registry, runtime

**Public API:**

- `pub struct WorkerExecutor`
- `impl WorkerExecutor :: fn new(engine : Arc < dyn WasmExecutor >) -> Self`
- `impl WorkerExecutor :: async fn execute(& self, task : TaskDefinition) -> Result <Vec <u8> , WasmError>`
- `pub struct Worker`
- `impl Worker :: fn new(receiver : mpsc :: Receiver < WorkerMessage >, executor : Arc < WorkerExecutor >, events : EventBus, persistence : Arc < PersistenceEngine >, retry_queue : Arc < PriorityTaskQueue >) -> Self`
- `impl Worker :: async fn run(mut self) -> Result <() , RuntimeError>`
- `pub struct CancellationRegistry`
- `impl CancellationRegistry :: fn new() -> Self`
- `impl CancellationRegistry :: fn register(& self, task_id : TaskId) -> CancellationToken`
- `impl CancellationRegistry :: fn cancel(& self, task_id : & TaskId)`
- `impl CancellationRegistry :: fn remove(& self, task_id : & TaskId)`
- `pub struct WorkerHandle`
- `pub struct WorkerManager`
- `impl WorkerManager :: fn new() -> Self`
- `impl WorkerManager :: fn spawn_workers(& mut self, count : usize, channel_capacity : usize, engine : Arc < dyn WasmExecutor >, events : EventBus, persistence : Arc < PersistenceEngine >, retry_queue : Arc < PriorityTaskQueue >)` — Worker'ları spawn et ve supervisor'ı arka planda başlat.
- `impl WorkerManager :: fn next_worker(& self) -> Option <WorkerHandle>` — Round-robin worker seçimi.
- `impl WorkerManager :: async fn send_shutdown_all(& self)` — Tüm worker'lara Shutdown mesajı gönder.
- `pub enum WorkerState`
- `pub enum WorkerMessage`
- `pub struct WorkerSpawnParams` — Worker spawn için gereken tüm bağımlılıklar. Supervisor, crashed worker'ı yeniden spawn etmek için bu parametreleri tutar.
- `pub struct WorkerSupervisor`
- `impl WorkerSupervisor :: fn new(params : WorkerSpawnParams, target_count : usize) -> Self`
- `impl WorkerSupervisor :: fn spawn_initial(& mut self)` — İlk worker'ları spawn et. WorkerManager yerine Supervisor artık spawn eder.
- `impl WorkerSupervisor :: fn senders(& self) -> & [tokio::sync::mpsc::Sender <WorkerMessage>]` — Dispatcher'ın task göndermesi için sender'ları al.
- `impl WorkerSupervisor :: async fn run(mut self, mut shutdown_rx : watch :: Receiver < bool >)` — Arka plan izleme döngüsünü başlat. Shutdown sinyali alınınca izlemeyi durdurur. Kalan worker'lar Shutdown mesajı aldıktan sonra temiz çıkar (WorkerManager.send_shutdown_all() ile).

### `workflows` (katman 2, 1650 LOC)

**Bağımlı olduğu:** agents, ai, errors, orchestration, runtime, task, types

**Kendisine bağımlı olanlar:** api, logging, orchestration, tests

**Public API:**

- `pub enum WorkflowState`
- `pub enum WorkflowEngineError`
- `pub struct WorkflowEngine`
- `impl WorkflowEngine :: fn compile(workflow : & WorkflowGraph) -> Result <WorkflowExecutionGraph , String>` — WorkflowGraph (struct) → validate → ExecutionGraph. Geriye dönük uyumluluk için korundu.
- `impl WorkflowEngine :: fn compile_dsl(dsl : & WorkflowDsl) -> Result <WorkflowExecutionGraph , WorkflowEngineError>` — WorkflowDsl → validate → ExecutionGraph.
- `impl WorkflowEngine :: fn from_json(json : & str) -> Result <WorkflowExecutionGraph , WorkflowEngineError>` — JSON string → compile → validate → ExecutionGraph.
- `impl WorkflowEngine :: async fn run_dsl(dsl : & WorkflowDsl, runtime : RuntimeHandle) -> Result <WorkflowState , WorkflowEngineError>` — compile → validate → execute tam pipeline.
- `impl WorkflowEngine :: async fn run_json(json : & str, runtime : RuntimeHandle) -> Result <WorkflowState , WorkflowEngineError>` — JSON → execute tam pipeline.
- `pub enum WorkflowPriority`
- `pub struct WorkflowScheduler`
- `impl WorkflowScheduler :: fn ready_nodes(graph : & WorkflowExecutionGraph, completed : & HashSet < Uuid >) -> Vec <Uuid>`
- `pub struct WorkflowDeadline`
- `pub struct WorkflowExecutor`
- `impl WorkflowExecutor :: fn new(runtime : RuntimeHandle) -> Self`
- `impl WorkflowExecutor :: async fn execute(& mut self, graph : WorkflowExecutionGraph) -> Result <() , RuntimeError>`
- `pub struct WorkflowRecovery`
- `pub struct WorkflowRecoveryPlan`
- `impl WorkflowRecovery :: fn resumable(checkpoint : & WorkflowCheckpoint) -> bool`
- `impl WorkflowRecovery :: async fn recover(checkpoint : WorkflowCheckpoint)`
- `pub struct WorkflowCheckpoint`
- `pub struct FailoverPlan`
- `pub struct WorkflowGraph`
- `pub struct WorkflowCheckpoint`
- `pub struct WorkflowCondition`
- `pub struct DispatchRequest`
- `pub struct WorkflowDispatcher`
- `impl WorkflowDispatcher :: fn dispatch(request : DispatchRequest) -> Uuid`
- `pub struct FanOutPolicy`
- `pub struct ParallelismPolicy`
- `pub struct WorkflowDsl` — Kullanıcının yazdığı workflow tanımı. JSON örneği: ```json { "name": "my-workflow", "steps": [ { "id": "step-1", "name": "fetch", "type": "wasm", "entrypoint": "fetch_data", "depends_on": [] }, { "id": "step-2", "name": "process", "type": "agent", "entrypoint": "process", "depends_on": ["step-1"] } ] } ```
- `pub struct StepDsl`
- `pub enum CompilerError`
- `pub struct WorkflowCompiler`
- `impl WorkflowCompiler :: fn compile_dsl(dsl : & WorkflowDsl) -> Result <WorkflowExecutionGraph , CompilerError>` — WorkflowDsl (JSON/YAML kaynaklı) → ExecutionGraph.
- `impl WorkflowCompiler :: fn compile(graph : & WorkflowGraph) -> WorkflowExecutionGraph` — WorkflowGraph (eski struct format) → ExecutionGraph. Geriye dönük uyumluluk.
- `impl WorkflowCompiler :: fn from_json(json : & str) -> Result <WorkflowExecutionGraph , String>` — JSON string → ExecutionGraph.
- `pub enum ResumeStrategy`
- `pub struct WorkflowTopology`
- `impl WorkflowTopology :: fn roots(graph : & WorkflowExecutionGraph) -> Vec <Uuid>`
- `impl WorkflowTopology :: fn leaves(graph : & WorkflowExecutionGraph) -> Vec <Uuid>`
- `impl WorkflowTopology :: fn adjacency(graph : & WorkflowExecutionGraph) -> HashMap <Uuid , Vec <Uuid> ,>`
- `pub enum ValidationError`
- `pub struct WorkflowValidator`
- `impl WorkflowValidator :: fn validate(graph : & WorkflowExecutionGraph) -> bool` — Hızlı boolean kontrol (geriye dönük uyumluluk).
- `impl WorkflowValidator :: fn validate_full(graph : & WorkflowExecutionGraph) -> Result <() , Vec <ValidationError>>` — Tam doğrulama — tüm hataları döndür.
- `pub enum CancellationStrategy`
- `pub trait WorkflowPersistence`
- `pub struct DistributedWorkflow`
- `pub struct QueuedWorkflow`
- `pub struct WorkflowQueue`
- `impl WorkflowQueue :: fn new() -> Self`
- `impl WorkflowQueue :: fn push(& mut self, workflow : QueuedWorkflow)`
- `impl WorkflowQueue :: fn pop(& mut self) -> Option <QueuedWorkflow ,>`
- `pub struct WorkflowEvent`


---

# Flutter Önyüz (flutter_app/lib/)

> Bu dosya implementasyon gövdesi içermez. Sadece mimari, modül amacı (doc comment) ve public API imzaları yer alır.

## Mimari

| Katman | Modül | LOC | Fan-in | Fan-out |
|---|---|---|---|---|
| 3 | src | 2410 | 3 | 0 |
| 2 | api | 121 | 1 | 1 |
| 1 | screens | 3709 | 1 | 3 |
| 2 | services | 204 | 1 | 0 |
| 0 | root | 116 | 0 | 2 |
| 0 | widgets | 0 | 0 | 0 |

## Modüller

### `api` (katman 2, 121 LOC)

**Bağımlı olduğu:** src

**Kendisine bağımlı olanlar:** screens

**Public API:**

- `class AetherApi`

### `root` (katman 0, 116 LOC)

**Bağımlı olduğu:** screens, src

**Public API:**

- `class AetherOSApp extends StatelessWidget` — Normal uygulama

### `screens` (katman 1, 3709 LOC)

**Bağımlı olduğu:** api, services, src

**Kendisine bağımlı olanlar:** root

**Public API:**

- `class HomeScreen extends ConsumerWidget`
- `class TaskListScreen extends StatefulWidget`
- `class LogScreen extends StatefulWidget`
- `class WasmModule`
- `class WasmModuleScreen extends StatefulWidget`
- `class ScriptEditorScreen extends StatefulWidget`
- `class AiSettingsScreen extends StatefulWidget`
- `class AiChatScreen extends StatefulWidget`
- `class BackupScreen extends StatefulWidget`
- `class SubmitTaskScreen extends StatefulWidget`

### `services` (katman 2, 204 LOC)

**Kendisine bağımlı olanlar:** screens

**Public API:**

- `enum MessageRole { user, model }`
- `class ChatMessage`
- `class GeminiService`
- `class GeminiException implements Exception`

### `src` (katman 3, 2410 LOC)

**Amaç:** Flutter'ın task göndermek için kullandığı tip.  wasm_module_hash: 64 karakter hex string (SHA-256) Boş string → modülsüz task (test için)

**Kendisine bağımlı olanlar:** api, root, screens

**Public API:**

- `class RustLib extends BaseEntrypoint<RustLibApi, RustLibApiImpl, RustLibWire>` — Main entrypoint of the Rust API
- `abstract class RustLibApi extends BaseApi`
- `class RustLibApiImpl extends RustLibApiImplPlatform implements RustLibApi`
- `class TaskRequest`
- `class TaskStatusResponse`
- `class MetricsSnapshot`
- `class RuntimeInfo`
- `class ModuleUploadResponse`
- `class LogRecord`
- `abstract class RustLibApiImplPlatform extends BaseApiImpl<RustLibWire>`
- `class RustLibWire implements BaseWire`
- `abstract class RustLibApiImplPlatform extends BaseApiImpl<RustLibWire>`
- `class RustLibWire implements BaseWire`
- `class RuntimeInfo`
- `class MetricsSnapshot`
- `class RegistryEntry`
- `class RuntimeId`

