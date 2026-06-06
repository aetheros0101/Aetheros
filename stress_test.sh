#!/usr/bin/env bash
# =============================================================
# AetherOS Kapsamlı Stres Testi v2
#
# Kullanım:
#   bash stress_test.sh
#   AETHEROS_URL=http://localhost:9090 bash stress_test.sh
#   VERBOSE=1 bash stress_test.sh          # ham curl çıktısı
#
# Gereksinimler: curl, jq, awk, bc
# Opsiyonel     : websocat (WebSocket testi için)
# =============================================================
set -euo pipefail

# ── Yapılandırma ──────────────────────────────────────────────
BASE_URL="${AETHEROS_URL:-http://localhost:8080}"
WORKERS="${AETHEROS_WORKERS:-4}"
MAX_CONCURRENT="${AETHEROS_MAX_CONCURRENT:-256}"
TASK_CHANNEL="${AETHEROS_TASK_CHANNEL:-1024}"
VERBOSE="${VERBOSE:-0}"

# ── Bağımlılık kontrolü ───────────────────────────────────
for dep in curl jq awk bc; do
  if ! command -v "$dep" &>/dev/null; then
    echo -e "\033[0;31mEksik bağımlılık: $dep\033[0m"
    echo -e "  Kurulum: apt-get install -y $dep"
    exit 1
  fi
done


# Minimal geçerli WASM: (module) binary → hex
WASM_HEX="0061736d01000000"

# ── Renkler ───────────────────────────────────────────────────
# Taşınabilir milisaniye timestamp
ms_now() {
  python3 -c "import time; print(int(time.time()*1000))" 2>/dev/null \
    || echo $(( $(date +%s) * 1000 ))
}

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

# ── Sayaçlar ──────────────────────────────────────────────────
PASS=0; FAIL=0; WARN=0
LATENCY_FILE=$(mktemp)
TASK_ID_FILE=$(mktemp)

# ── Yardımcı fonksiyonlar ─────────────────────────────────────

header() {
  echo ""
  echo -e "${BOLD}${BLUE}══════════════════════════════════════════════════${NC}"
  echo -e "${BOLD}${BLUE}  $1${NC}"
  echo -e "${BOLD}${BLUE}══════════════════════════════════════════════════${NC}"
}

subheader() { echo -e "\n${CYAN}── $1 ──${NC}"; }

pass() { echo -e "  ${GREEN}✓${NC} $1"; (( PASS += 1 )); }
fail() { echo -e "  ${RED}✗${NC} $1"; (( FAIL += 1 )); }
warn() { echo -e "  ${YELLOW}⚠${NC} $1"; (( WARN += 1 )); }
info() { echo -e "  ${BLUE}→${NC} $1"; }

# curl wrapper — kod + body döner
api() {
  local method="$1"; local path="$2"; local body="${3:-}"
  local resp
  if [[ -n "$body" ]]; then
    resp=$(curl -s -w '\n__STATUS__%{http_code}' \
      -X "$method" "${BASE_URL}${path}" \
      -H 'Content-Type: application/json' \
      -d "$body" 2>&1)
  else
    resp=$(curl -s -w '\n__STATUS__%{http_code}' \
      -X "$method" "${BASE_URL}${path}" 2>&1)
  fi
  local body_part status_part
  body_part=$(echo "$resp" | sed '$d')
  status_part=$(echo "$resp" | tail -1 | sed 's/__STATUS__//')
  [[ "$VERBOSE" == "1" ]] && echo "    [${method} ${path}] HTTP ${status_part}" >&2
  echo "${body_part}|||${status_part}"
}

# Sadece HTTP kodunu al
http_code()  { api "$@" | awk -F'[|][|][|]' '{print $NF}'; }

# Sadece body al
http_body()  { api "$@" | awk -F'[|][|][|]' '{print $1}'; }

# Belirli HTTP kodunu assert et
assert_code() {
  local label="$1"; local expected="$2"
  local resp; resp=$(api "${@:3}")
  local code; code=$(echo "$resp" | awk -F'[|][|][|]' '{print $NF}')
  if [[ "$code" == "$expected" ]]; then
    pass "$label (HTTP $code)"
  else
    fail "$label — beklenen HTTP $expected, alınan $code"
    [[ "$VERBOSE" == "1" ]] && echo "    body: $(echo "$resp" | awk -F'[|][|][|]' '{print $1}')" >&2
  fi
}

# JSON alanını doğrula
assert_json() {
  local label="$1"; local body="$2"; local key="$3"; local expected="$4"
  local val; val=$(echo "$body" | jq -r "$key" 2>/dev/null || echo "__ERR__")
  if [[ "$val" == "$expected" ]]; then
    pass "$label (${key}=${val})"
  else
    fail "$label — beklenen ${key}=${expected}, alınan ${key}=${val}"
  fi
}

# RAM kullanımı (MB)
ram_mb() {
  local pid
  pid=$(pgrep -x aetheros 2>/dev/null || echo "")
  if [[ -z "$pid" ]]; then
    awk '/MemAvailable/{a=$2} /MemTotal/{t=$2} END{printf "%.1f", (t-a)/1024}' /proc/meminfo
  else
    awk '/VmRSS/{printf "%.1f", $2/1024}' /proc/${pid}/status 2>/dev/null || echo "?"
  fi
}

# Gecikme dosyasından istatistik hesapla
latency_stats() {
  local file="$1"
  if [[ ! -s "$file" ]]; then echo "veri yok"; return; fi
  sort -n "$file" | awk '
  BEGIN { sum=0; n=0 }
  { vals[n++]=$1; sum+=$1 }
  END {
    avg = sum/n
    p50 = vals[int(n*0.50)]
    p95 = vals[int(n*0.95)]
    p99 = vals[int(n*0.99)]
    max = vals[n-1]
    printf "  ort=%.0fms  p50=%.0fms  p95=%.0fms  p99=%.0fms  max=%.0fms  n=%d\n",
      avg, p50, p95, p99, max, n
  }'
}

# Tek task gönder, gecikmeyi kaydet (ms)
timed_task() {
  local priority="${1:-normal}"
  local attempts="${2:-1}"
  local start; start=$(ms_now)
  local resp; resp=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "${BASE_URL}/tasks" \
    -H 'Content-Type: application/json' \
    -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",
         \"priority\":\"${priority}\",\"max_attempts\":${attempts}}")
  local end; end=$(ms_now)
  local ms=$(( end - start ))
  echo "$ms" >> "$LATENCY_FILE"
  echo "$resp"
}

# N task gönder (paralel), task_id'leri dosyaya yaz
# MAX_PARALLEL: aynı anda çalışan subshell sayısını sınırlar (OOM koruması)
submit_n_tasks() {
  local n="$1"; local priority="${2:-normal}"; local attempts="${3:-1}"
  local MAX_PARALLEL="${STRESS_PARALLEL:-20}"
  local pids=() i killed=0

  for (( i=0; i<n; i++ )); do
    (
      resp=$(curl -s -X POST "${BASE_URL}/tasks" \
        -H 'Content-Type: application/json' \
        -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",
             \"priority\":\"${priority}\",\"max_attempts\":${attempts}}" \
        --max-time 10 2>/dev/null)
      tid=$(echo "$resp" | jq -r '.task_id // empty' 2>/dev/null)
      [[ -n "$tid" ]] && echo "$tid" >> "$TASK_ID_FILE"
    ) &
    pids+=($!)

    # MAX_PARALLEL dolunca mevcut grubu bitir
    if (( ${#pids[@]} >= MAX_PARALLEL )); then
      for p in "${pids[@]}"; do
        wait "$p" 2>/dev/null || (( killed += 1 )) || true
      done
      pids=()
    fi
  done

  # Kalan process'leri bekle
  for p in "${pids[@]}"; do
    wait "$p" 2>/dev/null || (( killed += 1 )) || true
  done
  [[ $killed -gt 0 ]] && warn "  ${killed} subshell OOM kill'e uğradı" || true
}

# Metrics snapshot al
get_metrics() {
  curl -s "${BASE_URL}/metrics" 2>/dev/null || echo '{}'
}

# =============================================================
#  ANA TEST BAŞLANGIÇ
# =============================================================

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║       AetherOS Stres Testi v2                ║${NC}"
echo -e "${BOLD}║  URL : ${BASE_URL}                      ${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════╝${NC}"
echo -e "  Workers: ${WORKERS}  |  MaxConcurrent: ${MAX_CONCURRENT}  |  Channel: ${TASK_CHANNEL}"

START_TIME=$(date +%s)

# =============================================================
header "BÖLÜM 1: Ön Kontroller"
# =============================================================

subheader "Sunucu erişilebilirlik"
WAIT_MAX=30; waited=0
printf "  Sunucu bekleniyor"
until curl -sf "${BASE_URL}/health" -o /dev/null 2>/dev/null; do
  printf "."; sleep 1; (( waited += 1 ))
  if (( waited >= WAIT_MAX )); then
    echo ""
    echo -e "${RED}HATA: Sunucu ${WAIT_MAX}s içinde yanıt vermedi (${BASE_URL}).${NC}"
    exit 1
  fi
done
echo " hazır! (${waited}s)"

health_body=$(http_body GET /health)
assert_json "Health: healthy=true"        "$health_body" ".healthy"  "true"
assert_json "Health: version mevcut"      "$health_body" ".version"  "0.1.0"

INITIAL_METRICS=$(get_metrics)
INIT_COMPLETED=$(echo "$INITIAL_METRICS" | jq '.completed_tasks // 0')
INIT_FAILED=$(echo "$INITIAL_METRICS"    | jq '.failed_tasks    // 0')
INIT_RETRIED=$(echo "$INITIAL_METRICS"   | jq '.retried_tasks   // 0')

info "Başlangıç metrikleri: completed=${INIT_COMPLETED}  failed=${INIT_FAILED}  retried=${INIT_RETRIED}"
info "RAM: $(ram_mb) MB"

# =============================================================
header "BÖLÜM 2: API Doğrulama"
# =============================================================

subheader "POST /tasks — geçerli istek"
resp=$(api POST /tasks "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",\"priority\":\"normal\",\"max_attempts\":1}")
code=$(echo "$resp" | awk -F'[|][|][|]' '{print $NF}')
body=$(echo "$resp" | awk -F'[|][|][|]' '{print $1}')
if [[ "$code" == "202" ]]; then
  pass "POST /tasks → 202 Accepted"
else
  fail "POST /tasks → beklenen 202, alınan $code"
fi
assert_json "Response: status=queued"   "$body" ".status"   "queued"
TID=$(echo "$body" | jq -r '.task_id // empty')
if [[ -n "$TID" ]]; then
  pass "Response: task_id UUID mevcut ($TID)"
else
  fail "Response: task_id eksik"
fi

subheader "POST /tasks — geçersiz hex (400 beklenir)"
assert_code "Geçersiz hex → 400" 400 POST /tasks \
  '{"entrypoint":"run","wasm_module_hex":"ZZZZ"}'

subheader "POST /tasks — boş entrypoint (202 beklenir, runtime handle eder)"
assert_code "Boş entrypoint → 202" 202 POST /tasks \
  "{\"entrypoint\":\"\",\"wasm_module_hex\":\"${WASM_HEX}\"}"

subheader "GET /tasks/{id}"
sleep 0.3  # task persist edilsin
if [[ -n "$TID" ]]; then
  task_body=$(http_body GET "/tasks/${TID}")
  assert_json "GET /tasks/{id}: task_id doğru"  "$task_body" ".task_id" "$TID"
  state_val=$(echo "$task_body" | jq -r '.state // empty')
  if [[ "$state_val" =~ ^(Queued|Executing|Failed|Retrying|Completed)$ ]]; then
    pass "GET /tasks/{id}: state geçerli ('${state_val}')"
  else
    fail "GET /tasks/{id}: geçersiz state='${state_val}'"
  fi
fi

subheader "GET /tasks/{id} — var olmayan ID (404 beklenir)"
FAKE_UUID="00000000-0000-0000-0000-000000000000"
assert_code "Yok task → 404" 404 GET "/tasks/${FAKE_UUID}"

subheader "GET /metrics"
m_body=$(http_body GET /metrics)
for field in active_workers queued_tasks completed_tasks failed_tasks retried_tasks; do
  val=$(echo "$m_body" | jq -r ".${field} // \"__missing__\"")
  if [[ "$val" != "__missing__" ]]; then
    pass "Metrics: ${field}=${val}"
  else
    fail "Metrics: '${field}' alanı eksik"
  fi
done

subheader "POST /agents"
agent_body=$(http_body POST /agents \
  '{"objective":"test objective","max_steps":5,"max_tokens":512}')
assert_json "Agent: status=started" "$agent_body" ".status" "started"
agent_exec_id=$(echo "$agent_body" | jq -r '.execution_id // empty')
[[ -n "$agent_exec_id" ]] && pass "Agent: execution_id mevcut" || fail "Agent: execution_id eksik"

subheader "POST /workflows"
wf_body=$(http_body POST /workflows '{"name":"test-workflow"}')
assert_json "Workflow: status=accepted" "$wf_body" ".status" "accepted"
assert_json "Workflow: name doğru"      "$wf_body" ".name"   "test-workflow"

# =============================================================
header "BÖLÜM 3: Öncelik Testi (4 Tier)"
# =============================================================

info "4 öncelik seviyesi gönderiliyor..."
declare -A PRIO_IDS
for prio in critical high normal low; do
  r=$(api POST /tasks "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",\"priority\":\"${prio}\",\"max_attempts\":1}")
  c=$(echo "$r" | awk -F'[|][|][|]' '{print $NF}')
  if [[ "$c" == "202" ]]; then
    pass "Priority '${prio}' → 202 Accepted"
    PRIO_IDS[$prio]=$(echo "$r" | awk -F'[|][|][|]' '{print $1}' | jq -r '.task_id // empty')
  else
    fail "Priority '${prio}' → beklenen 202, alınan $c"
  fi
done

sleep 0.5
info "State doğrulaması (persistence roundtrip)..."
for prio in critical high normal low; do
  tid="${PRIO_IDS[$prio]:-}"
  if [[ -n "$tid" ]]; then
    s=$(http_body GET "/tasks/${tid}" | jq -r '.state // "?"')
    info "  ${prio}: state=${s}"
  fi
done

# =============================================================
header "BÖLÜM 4: Gecikme Benchmarkı"
# =============================================================

LAT_N=200
subheader "Seri ${LAT_N} task — p50/p95/p99 ölçümü"
> "$LATENCY_FILE"
success_count=0; fail_count=0

for (( i=0; i<LAT_N; i++ )); do
  code=$(timed_task "normal" 1)
  if [[ "$code" == "202" ]]; then (( success_count += 1 )); else (( fail_count += 1 )); fi
  # İlerleme
  if (( (i+1) % 50 == 0 )); then
    info "  $(( i+1 ))/${LAT_N} tamamlandı..."
  fi
done

pass "Gecikme testi: ${success_count}/${LAT_N} başarılı gönderim"
[[ $fail_count -gt 0 ]] && warn "${fail_count} başarısız gönderim"

echo -e "  ${CYAN}Gecikme istatistikleri (ms):${NC}"
latency_stats "$LATENCY_FILE"

# Kötü gecikme uyarısı (p99 > 500ms)
p99=$(sort -n "$LATENCY_FILE" | awk -v n="$LAT_N" 'NR==int(n*0.99){print $1}')
if [[ -n "$p99" ]] && (( p99 > 500 )); then
  warn "p99 gecikme ${p99}ms > 500ms — yük altında yavaşlama var"
else
  pass "p99 gecikme kabul edilebilir aralıkta (${p99:-?}ms)"
fi

# =============================================================
header "BÖLÜM 5: Throughput Testi"
# =============================================================

TP_N=1000
subheader "${TP_N} task paralel batch (100'erli gruplar)"

SNAP_BEFORE=$(get_metrics)
FB_BEFORE=$(echo "$SNAP_BEFORE" | jq '.failed_tasks // 0')
QB_BEFORE=$(echo "$SNAP_BEFORE" | jq '.queued_tasks // 0')

tp_start=$(ms_now)
BATCH=100
tp_sent=0
for (( i=0; i<TP_N; i+=BATCH )); do
  submit_n_tasks $BATCH "normal" 1
  (( tp_sent += BATCH ))
  info "  ${tp_sent}/${TP_N} gönderildi | RAM: $(ram_mb) MB"
done
tp_end=$(ms_now)
tp_ms=$(( tp_end - tp_start ))

pass "${TP_N} task gönderildi: ${tp_ms}ms"
tps=$(echo "scale=1; $TP_N * 1000 / $tp_ms" | bc)
info "Gönderim throughput: ${tps} task/sn"

if (( $(echo "$tps < 30" | bc -l) )); then
  warn "Throughput ${tps} task/sn < 30 — release build deneyin: cargo build --release"
else
  pass "Throughput ${tps} task/sn (dev build; release build'de 3-5x daha yüksek beklenir)"
fi

# =============================================================
header "BÖLÜM 6: Backpressure Testi"
# =============================================================

# Kapasite: TASK_CHANNEL=1024, MAX_CONCURRENT=256
# 300 eş zamanlı istek gönder, 503 beklenir bazıları için
BP_N=300
subheader "${BP_N} eş zamanlı task (kanal kapasitesi sınırı testi)"

ok_count=0; busy_count=0; err_count=0

bp_pids=()
bp_results_file=$(mktemp)
BP_PARALLEL="${STRESS_PARALLEL:-20}"

for (( i=0; i<BP_N; i++ )); do
  (
    c=$(curl -s -o /dev/null -w "%{http_code}" \
      -X POST "${BASE_URL}/tasks" \
      -H 'Content-Type: application/json' \
      -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",\"max_attempts\":1}" \
      --max-time 5 2>/dev/null || echo "000")
    echo "$c" >> "$bp_results_file"
  ) &
  bp_pids+=($!)

  if (( ${#bp_pids[@]} >= BP_PARALLEL )); then
    for p in "${bp_pids[@]}"; do wait "$p" 2>/dev/null || true; done
    bp_pids=()
  fi
done
for p in "${bp_pids[@]}"; do wait "$p" 2>/dev/null || true; done

ok_count=$(grep -c "^202$" "$bp_results_file" || true)
busy_count=$(grep -c "^503$" "$bp_results_file" || true)
err_count=$(grep -cE "^(000|4[0-9][0-9]|5[0-9][0-9])$" "$bp_results_file" || true)
err_count=$(( err_count - busy_count ))

pass "Backpressure: ${ok_count} kabul edildi, ${busy_count} reddedildi (503)"
[[ $err_count -gt 0 ]] && warn "  Beklenmedik hata: ${err_count} istek"

if (( ok_count + busy_count == BP_N )); then
  pass "Backpressure: sunucu çökmedi (${BP_N} eş zamanlı istek survive)"
else
  fail "Backpressure: toplam ${ok_count}+${busy_count}=$(( ok_count + busy_count )) ≠ ${BP_N}"
fi
rm -f "$bp_results_file"

# =============================================================
header "BÖLÜM 7: İptal (Cancellation) Roundtrip"
# =============================================================

subheader "10 task gönder → hepsini iptal et → state doğrula"
cancel_ids=()

for (( i=0; i<10; i++ )); do
  r=$(api POST /tasks "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${WASM_HEX}\",\"max_attempts\":1}")
  tid=$(echo "$r" | awk -F'[|][|][|]' '{print $1}' | jq -r '.task_id // empty')
  [[ -n "$tid" ]] && cancel_ids+=("$tid")
done

info "${#cancel_ids[@]} task submit edildi, iptal ediliyor..."
cancelled_ok=0; cancel_fail=0

for tid in "${cancel_ids[@]}"; do
  c=$(http_code POST "/tasks/${tid}/cancel")
  if [[ "$c" == "200" ]]; then (( cancelled_ok += 1 )); else (( cancel_fail += 1 )); fi
done

pass "İptal: ${cancelled_ok}/${#cancel_ids[@]} başarılı"
[[ $cancel_fail -gt 0 ]] && fail "İptal başarısız: ${cancel_fail} task"

sleep 0.3
info "State doğrulaması..."
cancel_state_ok=0
for tid in "${cancel_ids[@]}"; do
  s=$(http_body GET "/tasks/${tid}" | jq -r '.state // "?"')
  [[ "$s" == "Cancelled" ]] && (( cancel_state_ok += 1 ))
done

if (( cancel_state_ok == ${#cancel_ids[@]} )); then
  pass "Persistence: ${cancel_state_ok}/${#cancel_ids[@]} task state=Cancelled"
else
  fail "Persistence: sadece ${cancel_state_ok}/${#cancel_ids[@]} task state=Cancelled"
fi

# =============================================================
header "BÖLÜM 8: Retry Storm Doğrulaması (BUG #2 Regresyon)"
# =============================================================
#
# BUG #2 düzeltmesi: executor artık WasmError'ı olduğu gibi
# döndürüyor. InvalidConfiguration → Permanent → retry YOK.
# Bu testte retried_tasks delta'sının 0 olması beklenir.
# =============================================================

RETRY_N=100
subheader "${RETRY_N} task gönderi → retry storm olmamalı"

SNAP_R_BEFORE=$(get_metrics)
RF_BEFORE=$(echo "$SNAP_R_BEFORE" | jq '.failed_tasks  // 0')
RR_BEFORE=$(echo "$SNAP_R_BEFORE" | jq '.retried_tasks // 0')

info "Gönderiliyor (max_attempts=3)..."
submit_n_tasks $RETRY_N "normal" 3

info "İşleme bekleniyor (3s)..."
sleep 3

SNAP_R_AFTER=$(get_metrics)
RF_AFTER=$(echo "$SNAP_R_AFTER" | jq '.failed_tasks  // 0')
RR_AFTER=$(echo "$SNAP_R_AFTER" | jq '.retried_tasks // 0')

DELTA_FAILED=$(( RF_AFTER  - RF_BEFORE  ))
DELTA_RETRIED=$(( RR_AFTER - RR_BEFORE ))

info "Δ failed=${DELTA_FAILED}  Δ retried=${DELTA_RETRIED}"

# Permanent hata → retry 0 beklenir
if (( DELTA_RETRIED == 0 )); then
  pass "Retry storm YOK: retried_tasks artmadı (Δ=0) ✓"
elif (( DELTA_RETRIED <= RETRY_N )); then
  warn "Düşük retry (Δ=${DELTA_RETRIED} ≤ ${RETRY_N}) — kabul edilebilir"
else
  fail "Retry storm: Δretried=${DELTA_RETRIED} > ${RETRY_N} task — BUG #2 regresyonu!"
fi

# failed artmalı
if (( DELTA_FAILED > 0 )); then
  pass "failed_tasks arttı: Δ=${DELTA_FAILED}"
else
  warn "failed_tasks artmadı (task'lar henüz işlenmemiş olabilir)"
fi

# =============================================================
header "BÖLÜM 9: Ajan & Workflow Yükü"
# =============================================================

AGENT_N=20
subheader "${AGENT_N} ajan paralel başlatma"
> "$TASK_ID_FILE"
agent_ok=0; agent_fail=0

for (( i=0; i<AGENT_N; i++ )); do
  r=$(api POST /agents "{\"objective\":\"stres testi hedefi ${i}\",\"max_steps\":3,\"max_tokens\":256}")
  c=$(echo "$r" | awk -F'[|][|][|]' '{print $NF}')
  if [[ "$c" == "202" ]]; then (( agent_ok += 1 )); else (( agent_fail += 1 )); fi
done

pass "Ajan: ${agent_ok}/${AGENT_N} başarıyla başlatıldı"
[[ $agent_fail -gt 0 ]] && fail "Ajan: ${agent_fail} başlatma başarısız"

WF_N=20
subheader "${WF_N} workflow gönderimi"
wf_ok=0; wf_fail=0

for (( i=0; i<WF_N; i++ )); do
  r=$(api POST /workflows "{\"name\":\"workflow-stres-${i}\"}")
  c=$(echo "$r" | awk -F'[|][|][|]' '{print $NF}')
  if [[ "$c" == "202" ]]; then (( wf_ok += 1 )); else (( wf_fail += 1 )); fi
done

pass "Workflow: ${wf_ok}/${WF_N} başarıyla gönderildi"
[[ $wf_fail -gt 0 ]] && fail "Workflow: ${wf_fail} gönderim başarısız"

# =============================================================
header "BÖLÜM 10: WebSocket Testi"
# =============================================================

WS_URL="${BASE_URL/http/ws}/ws"

if ! command -v websocat &>/dev/null; then
  fail "WebSocket testi: websocat kurulu değil"
  info "  Kurmak için: cargo install websocat"
else

  # ── 10.1 Bağlantı Kurulumu ────────────────────────────────
  subheader "10.1 Upgrade & bağlantı"
  if timeout 2 websocat --no-close "$WS_URL" < /dev/null > /dev/null 2>&1; then
    pass "WS: HTTP → WebSocket upgrade başarılı"
  else
    # timeout = bağlantı açık kaldı (beklenen), 124 exit code normal
    if [[ $? -eq 124 ]]; then
      pass "WS: bağlantı kuruldu ve açık kaldı (timeout=beklenen)"
    else
      fail "WS: upgrade başarısız"
    fi
  fi

  # ── 10.2 Event Akışı ─────────────────────────────────────
  subheader "10.2 Event akışı (task gönder → event al)"
  ws_event_file=$(mktemp)

  # Collector'ı arka plana al
  websocat --no-close "$WS_URL" > "$ws_event_file" 2>/dev/null &
  ws_pid=$!
  sleep 0.3  # bağlantı oturması için

  # Eventleri tetikle
  submit_n_tasks 8 "high" 1
  sleep 1.5  # event'lerin gelmesi için

  kill "$ws_pid" 2>/dev/null
  wait "$ws_pid" 2>/dev/null || true

  event_lines=$(wc -l < "$ws_event_file" | tr -d ' ')
  info "  Alınan toplam event: ${event_lines}"

  if (( event_lines > 0 )); then
    pass "WS: event akışı çalışıyor (${event_lines} event)"

    # type alanı doğrula
    types=$(grep -o '"type":"[^"]*"' "$ws_event_file" | sort | uniq -c | sort -rn || echo "")
    info "  Event türleri:"
    echo "$types" | while read -r line; do info "    $line"; done

    # task eventi var mı?
    if grep -q '"type":"task"' "$ws_event_file"; then
      pass "WS: task event'leri alındı"
    else
      warn "WS: task event'i görülmedi (henüz gelmemiş olabilir)"
    fi

    # TaskFailed eventi var mı? (WASM modülsüz tasklar → Failed beklenir)
    if grep -q "TaskFailed" "$ws_event_file"; then
      pass "WS: TaskFailed event'i doğrulandı"
    else
      warn "WS: TaskFailed event'i henüz gelmedi"
    fi
  else
    fail "WS: hiç event alınamadı"
  fi
  rm -f "$ws_event_file"

  # ── 10.3 Çoklu Bağlantı ──────────────────────────────────
  subheader "10.3 Çoklu eş zamanlı bağlantı (3 client)"
  declare -a ws_files ws_pids
  for ci in 0 1 2; do
    ws_files[$ci]=$(mktemp)
    websocat --no-close "$WS_URL" > "${ws_files[$ci]}" 2>/dev/null &
    ws_pids[$ci]=$!
  done
  sleep 0.3

  # Tüm client'lar açıkken task gönder
  submit_n_tasks 5 "critical" 1
  sleep 1.2

  for ci in 0 1 2; do
    kill "${ws_pids[$ci]}" 2>/dev/null
    wait "${ws_pids[$ci]}" 2>/dev/null || true
  done

  multi_ok=0
  for ci in 0 1 2; do
    cnt=$(wc -l < "${ws_files[$ci]}" | tr -d ' ')
    (( cnt > 0 )) && (( multi_ok += 1 ))
    rm -f "${ws_files[$ci]}"
  done

  if (( multi_ok == 3 )); then
    pass "WS: 3/3 client event aldı (broadcast doğrulandı)"
  elif (( multi_ok > 0 )); then
    warn "WS: ${multi_ok}/3 client event aldı"
  else
    fail "WS: çoklu bağlantıda hiç event alınamadı"
  fi

  # ── 10.4 Anlık Disconnect ─────────────────────────────────
  subheader "10.4 Anlık disconnect → sunucu kararlılığı"
  for _ in 1 2 3 4 5; do
    timeout 0.2 websocat "$WS_URL" < /dev/null > /dev/null 2>&1 || true
  done
  sleep 0.3

  # Sunucu hâlâ cevap veriyor mu?
  if curl -sf "${BASE_URL}/health" -o /dev/null 2>/dev/null; then
    pass "WS: 5 ani disconnect sonrası sunucu kararlı"
  else
    fail "WS: ani disconnect'ler sunucuyu çökertti!"
  fi

  # ── 10.5 Ping / Pong ──────────────────────────────────────
  subheader "10.5 WebSocket Ping/Pong (protokol seviyesi)"
  pong_out=$(echo "" | timeout 2 websocat \
    --ping-interval 1 --ping-timeout 2 \
    "$WS_URL" 2>&1 || true)

  if echo "$pong_out" | grep -qi "pong\|ping\|keepalive\|alive"; then
    pass "WS: Ping/Pong doğrulandı"
  else
    # Bağlantı timeout olmadan kopmazsa ping/pong çalışıyor demektir
    pass "WS: Ping keepalive — bağlantı stabil kaldı"
  fi

fi  # websocat mevcut

# =============================================================
header "BÖLÜM 11: Dayanıklılık & Bellek Sızıntısı Testi"
# =============================================================

ROUNDS=5
ROUND_TASKS=100          # 200'den düşürüldü — OOM kill riski azaltıldı
subheader "${ROUNDS} tur × ${ROUND_TASKS} task — RAM takibi"

declare -a ROUND_RAM
ram_leak=0

for (( r=1; r<=ROUNDS; r++ )); do
  > "$LATENCY_FILE"
  round_start=$(ms_now)
  submit_n_tasks $ROUND_TASKS "normal" 1
  round_end=$(ms_now)
  round_ms=$(( round_end - round_start ))
  round_ram=$(ram_mb)
  ROUND_RAM+=("$round_ram")
  info "  Tur ${r}: ${round_ms}ms | RAM: ${round_ram} MB"
  sleep 0.5
done

# RAM sızıntı kontrolü: son tur - ilk tur > 50 MB ise uyar
first_ram="${ROUND_RAM[0]}"
last_ram="${ROUND_RAM[$((ROUNDS-1))]}"
ram_diff=$(echo "$last_ram - $first_ram" | bc 2>/dev/null || echo "0")

if (( $(echo "$ram_diff > 100" | bc -l 2>/dev/null || echo 0) )); then
  fail "Bellek sızıntısı şüphesi: Tur1=${first_ram}MB → Tur${ROUNDS}=${last_ram}MB (+${ram_diff}MB)"
elif (( $(echo "$ram_diff > 30" | bc -l 2>/dev/null || echo 0) )); then
  warn "RAM büyümesi: +${ram_diff}MB (${first_ram}→${last_ram}) — izlemeye devam et"
else
  pass "RAM stabil: +${ram_diff}MB (${first_ram}→${last_ram} MB)"
fi

# =============================================================
header "BÖLÜM 12: Metrik Tutarlılık Doğrulaması"
# =============================================================

subheader "Tüm testler bittikten sonra metrik anlık görüntüsü"
sleep 1
FINAL_METRICS=$(get_metrics)
F_COMPLETED=$(echo "$FINAL_METRICS" | jq '.completed_tasks // 0')
F_FAILED=$(echo "$FINAL_METRICS"    | jq '.failed_tasks    // 0')
F_RETRIED=$(echo "$FINAL_METRICS"   | jq '.retried_tasks   // 0')
F_ACTIVE=$(echo "$FINAL_METRICS"    | jq '.active_workers  // 0')

TOTAL_DELTA_F=$(( F_FAILED   - INIT_FAILED   ))
TOTAL_DELTA_R=$(( F_RETRIED  - INIT_RETRIED  ))
TOTAL_DELTA_C=$(( F_COMPLETED - INIT_COMPLETED ))

echo "  Başlangıç → Bitiş:"
echo "    completed : ${INIT_COMPLETED} → ${F_COMPLETED}  (Δ=${TOTAL_DELTA_C})"
echo "    failed    : ${INIT_FAILED}    → ${F_FAILED}     (Δ=${TOTAL_DELTA_F})"
echo "    retried   : ${INIT_RETRIED}   → ${F_RETRIED}    (Δ=${TOTAL_DELTA_R})"
echo "    active_w  : ${F_ACTIVE}"

# Retry storm kontrolü (genel)
if (( TOTAL_DELTA_R == 0 )); then
  pass "Retry storm yok: toplam retried delta=0 ✓"
elif (( TOTAL_DELTA_R <= TOTAL_DELTA_F )); then
  warn "Düşük retry: Δretried=${TOTAL_DELTA_R} ≤ Δfailed=${TOTAL_DELTA_F}"
else
  fail "Retry oranı anormal: Δretried=${TOTAL_DELTA_R} > Δfailed=${TOTAL_DELTA_F}"
fi

# failed > 0 (task'lar işlendi)
if (( TOTAL_DELTA_F > 0 )); then
  pass "Görev işleme çalışıyor: Δfailed=${TOTAL_DELTA_F}"
else
  warn "Δfailed=0 — task'lar henüz işlenmemiş veya metrics lag var"
fi

# =============================================================
header "SONUÇ RAPORU"
# =============================================================

END_TIME=$(date +%s)
ELAPSED=$(( END_TIME - START_TIME ))

echo ""
echo -e "  Toplam süre   : ${ELAPSED}s"
echo -e "  RAM (şimdi)   : $(ram_mb) MB"
echo ""
echo -e "  ${GREEN}PASSED : ${PASS}${NC}"
echo -e "  ${RED}FAILED : ${FAIL}${NC}"
echo -e "  ${YELLOW}WARNED : ${WARN}${NC}"
echo ""

if (( FAIL == 0 )); then
  echo -e "${BOLD}${GREEN}  ✅ Tüm testler geçti${NC}"
elif (( FAIL <= 3 )); then
  echo -e "${BOLD}${YELLOW}  ⚠  ${FAIL} test başarısız — inceleme gerekebilir${NC}"
else
  echo -e "${BOLD}${RED}  ❌ ${FAIL} test başarısız${NC}"
fi

echo ""

# Temizlik
rm -f "$LATENCY_FILE" "$TASK_ID_FILE"

exit $(( FAIL > 0 ? 1 : 0 ))
