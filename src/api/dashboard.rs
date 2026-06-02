// ============================================================
// src/api/dashboard.rs  (YENİ)
//
// Sprint 8: AetherOS Realtime Dashboard
//
// /dashboard → tek sayfalık HTML (inline CSS + JS)
// /ws        → WebSocket → EventBus stream
//
// Özellikler:
//   - Realtime task akışı (Queued/Executing/Done/Failed)
//   - Metrik kartlar (total, active, failed, retried)
//   - Son 50 event log
//   - Worker durumu
//   - Runtime health (yeşil/kırmızı)
//   - WebSocket bağlantı durumu (auto-reconnect)
// ============================================================

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};

/// GET /dashboard → HTML dashboard
pub async fn dashboard_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        DASHBOARD_HTML,
    )
}

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>AetherOS Dashboard</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'Segoe UI', system-ui, sans-serif;
    background: #0a0e1a;
    color: #e2e8f0;
    min-height: 100vh;
  }
  header {
    background: #111827;
    border-bottom: 1px solid #1e293b;
    padding: 16px 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .logo {
    font-size: 1.25rem;
    font-weight: 700;
    color: #60a5fa;
    letter-spacing: 0.05em;
  }
  .logo span { color: #a78bfa; }
  .ws-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8rem;
    color: #94a3b8;
  }
  .ws-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: #ef4444;
    transition: background 0.3s;
  }
  .ws-dot.connected { background: #22c55e; }

  main { padding: 24px; max-width: 1400px; margin: 0 auto; }

  /* Metrik Kartlar */
  .metrics {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }
  .card {
    background: #111827;
    border: 1px solid #1e293b;
    border-radius: 12px;
    padding: 20px;
  }
  .card-label {
    font-size: 0.75rem;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 8px;
  }
  .card-value {
    font-size: 2rem;
    font-weight: 700;
    line-height: 1;
  }
  .card-value.blue   { color: #60a5fa; }
  .card-value.green  { color: #22c55e; }
  .card-value.red    { color: #ef4444; }
  .card-value.yellow { color: #f59e0b; }
  .card-value.purple { color: #a78bfa; }

  /* Grid Layout */
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }

  .panel {
    background: #111827;
    border: 1px solid #1e293b;
    border-radius: 12px;
    overflow: hidden;
  }
  .panel-header {
    padding: 14px 18px;
    border-bottom: 1px solid #1e293b;
    font-size: 0.85rem;
    font-weight: 600;
    color: #94a3b8;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .badge {
    background: #1e293b;
    border-radius: 9999px;
    padding: 2px 8px;
    font-size: 0.75rem;
  }

  /* Event Log */
  #event-log {
    height: 320px;
    overflow-y: auto;
    padding: 8px 0;
    scrollbar-width: thin;
    scrollbar-color: #1e293b transparent;
  }
  .event-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 6px 18px;
    border-bottom: 1px solid #0f172a;
    font-size: 0.8rem;
    animation: fadeIn 0.2s ease;
  }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; } }
  .event-time { color: #475569; min-width: 70px; font-family: monospace; }
  .event-type { min-width: 80px; font-weight: 600; }
  .event-type.task     { color: #60a5fa; }
  .event-type.runtime  { color: #22c55e; }
  .event-type.worker   { color: #a78bfa; }
  .event-type.telemetry{ color: #f59e0b; }
  .event-body { color: #94a3b8; flex: 1; word-break: break-all; }

  /* Task State */
  #task-list {
    height: 320px;
    overflow-y: auto;
    padding: 8px 0;
  }
  .task-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 18px;
    border-bottom: 1px solid #0f172a;
    font-size: 0.8rem;
  }
  .state-badge {
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 0.7rem;
    font-weight: 600;
    min-width: 70px;
    text-align: center;
  }
  .state-queued    { background: #1e3a5f; color: #60a5fa; }
  .state-executing { background: #1a3a2a; color: #22c55e; }
  .state-completed { background: #14532d; color: #4ade80; }
  .state-failed    { background: #450a0a; color: #f87171; }
  .state-retrying  { background: #422006; color: #fbbf24; }
  .state-cancelled { background: #1e1b4b; color: #818cf8; }
  .task-id { color: #475569; font-family: monospace; font-size: 0.7rem; }

  /* Sparkline */
  .sparkline-bar {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 40px;
    padding: 0 18px 8px;
  }
  .spark-col {
    flex: 1;
    background: #1e40af;
    border-radius: 2px 2px 0 0;
    min-height: 2px;
    transition: height 0.3s;
  }
</style>
</head>
<body>

<header>
  <div class="logo">Aether<span>OS</span></div>
  <div class="ws-status">
    <div class="ws-dot" id="ws-dot"></div>
    <span id="ws-label">Bağlanıyor...</span>
  </div>
</header>

<main>
  <!-- Metrik Kartlar -->
  <div class="metrics">
    <div class="card">
      <div class="card-label">Toplam Task</div>
      <div class="card-value blue" id="m-total">0</div>
    </div>
    <div class="card">
      <div class="card-label">Aktif</div>
      <div class="card-value green" id="m-active">0</div>
    </div>
    <div class="card">
      <div class="card-label">Tamamlandı</div>
      <div class="card-value purple" id="m-completed">0</div>
    </div>
    <div class="card">
      <div class="card-label">Başarısız</div>
      <div class="card-value red" id="m-failed">0</div>
    </div>
    <div class="card">
      <div class="card-label">Retry</div>
      <div class="card-value yellow" id="m-retried">0</div>
    </div>
    <div class="card">
      <div class="card-label">Event/sn</div>
      <div class="card-value blue" id="m-eps">0</div>
    </div>
  </div>

  <!-- Sparkline -->
  <div class="panel" style="margin-bottom:16px">
    <div class="panel-header">
      <span>Task Aktivitesi</span>
      <span class="badge" id="spark-label">son 30s</span>
    </div>
    <div class="sparkline-bar" id="sparkline">
    </div>
  </div>

  <!-- Ana Grid -->
  <div class="grid">
    <!-- Event Log -->
    <div class="panel">
      <div class="panel-header">
        <span>Canlı Event Log</span>
        <span class="badge" id="event-count">0</span>
      </div>
      <div id="event-log"></div>
    </div>

    <!-- Task Durumları -->
    <div class="panel">
      <div class="panel-header">
        <span>Son Task'lar</span>
        <span class="badge" id="task-count">0</span>
      </div>
      <div id="task-list"></div>
    </div>
  </div>
</main>

<script>
// ── State ─────────────────────────────────────────────────
const state = {
  metrics: { total: 0, active: 0, completed: 0, failed: 0, retried: 0 },
  events: [],
  tasks: new Map(),   // task_id → { state, updated }
  spark: new Array(30).fill(0),
  eps: 0,
  epsWindow: 0,
  maxEvents: 50,
  maxTasks: 50,
};

// ── WebSocket ─────────────────────────────────────────────
let ws = null;
let reconnectTimer = null;

function connect() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}/ws`;
  ws = new WebSocket(url);

  ws.onopen = () => {
    document.getElementById('ws-dot').classList.add('connected');
    document.getElementById('ws-label').textContent = 'Bağlı';
    if (reconnectTimer) { clearTimeout(reconnectTimer); reconnectTimer = null; }
  };

  ws.onclose = () => {
    document.getElementById('ws-dot').classList.remove('connected');
    document.getElementById('ws-label').textContent = 'Yeniden bağlanıyor...';
    reconnectTimer = setTimeout(connect, 3000);
  };

  ws.onerror = () => ws.close();

  ws.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      handleEvent(msg);
    } catch (_) {}
  };
}

// ── Event İşleme ─────────────────────────────────────────
function handleEvent(msg) {
  const { type, event } = msg;
  const now = new Date();

  // EPS sayacı
  state.epsWindow++;

  // Event log
  state.events.unshift({ type, event, time: now });
  if (state.events.length > state.maxEvents) state.events.pop();

  // Sparkline
  state.spark[state.spark.length - 1]++;

  // Metrik + task state
  if (type === 'task') {
    state.metrics.total++;
    updateTaskMetrics(event);
  } else if (type === 'runtime') {
    // Runtime event — log yeterli
  }

  renderAll();
}

function updateTaskMetrics(event) {
  // event: "TaskCompleted { task_id: ... }"
  const taskId = extractTaskId(event);

  if (event.includes('TaskQueued')) {
    state.metrics.active++;
    state.tasks.set(taskId, { state: 'queued', id: taskId });
  } else if (event.includes('TaskStarted')) {
    state.tasks.set(taskId, { state: 'executing', id: taskId });
  } else if (event.includes('TaskCompleted')) {
    state.metrics.active = Math.max(0, state.metrics.active - 1);
    state.metrics.completed++;
    state.tasks.set(taskId, { state: 'completed', id: taskId });
  } else if (event.includes('TaskFailed')) {
    state.metrics.active = Math.max(0, state.metrics.active - 1);
    state.metrics.failed++;
    state.tasks.set(taskId, { state: 'failed', id: taskId });
  } else if (event.includes('TaskRetried')) {
    state.metrics.retried++;
    state.tasks.set(taskId, { state: 'retrying', id: taskId });
  } else if (event.includes('TaskCancelled')) {
    state.metrics.active = Math.max(0, state.metrics.active - 1);
    state.tasks.set(taskId, { state: 'cancelled', id: taskId });
  }

  // Max 50 task tut
  if (state.tasks.size > state.maxTasks) {
    const firstKey = state.tasks.keys().next().value;
    state.tasks.delete(firstKey);
  }
}

function extractTaskId(event) {
  const match = event.match(/task_id: ([a-f0-9-]+)/);
  return match ? match[1] : 'unknown';
}

// ── Render ────────────────────────────────────────────────
function renderAll() {
  // Metrikler
  document.getElementById('m-total').textContent = state.metrics.total;
  document.getElementById('m-active').textContent = state.metrics.active;
  document.getElementById('m-completed').textContent = state.metrics.completed;
  document.getElementById('m-failed').textContent = state.metrics.failed;
  document.getElementById('m-retried').textContent = state.metrics.retried;
  document.getElementById('m-eps').textContent = state.eps.toFixed(1);

  // Event log
  const log = document.getElementById('event-log');
  log.innerHTML = state.events.map(e => `
    <div class="event-item">
      <span class="event-time">${formatTime(e.time)}</span>
      <span class="event-type ${e.type}">${e.type}</span>
      <span class="event-body">${e.event}</span>
    </div>
  `).join('');
  document.getElementById('event-count').textContent = state.events.length;

  // Task listesi
  const tasks = [...state.tasks.values()].reverse();
  const taskList = document.getElementById('task-list');
  taskList.innerHTML = tasks.map(t => `
    <div class="task-item">
      <span class="state-badge state-${t.state}">${t.state}</span>
      <span class="task-id">${t.id.slice(0, 8)}…</span>
    </div>
  `).join('');
  document.getElementById('task-count').textContent = tasks.length;

  // Sparkline
  const max = Math.max(...state.spark, 1);
  const spark = document.getElementById('sparkline');
  spark.innerHTML = state.spark.map(v => `
    <div class="spark-col" style="height:${Math.max(2, (v/max)*36)}px"></div>
  `).join('');
}

function formatTime(d) {
  return d.toTimeString().slice(0, 8);
}

// ── Tick: EPS + Sparkline ─────────────────────────────────
setInterval(() => {
  state.eps = state.epsWindow;
  state.epsWindow = 0;

  // Sparkline kaydır
  state.spark.shift();
  state.spark.push(0);

  renderAll();
}, 1000);

// ── Başlat ────────────────────────────────────────────────
connect();
renderAll();
</script>
</body>
</html>"#;
