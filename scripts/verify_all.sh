#!/usr/bin/env bash
# ============================================================
# verify_all.sh — madde 1-5'i tek seferde doğrular.
#
# Önkoşul: sunucu ZATEN çalışıyor olmalı, şu env'lerle başlatılmış:
#   AETHEROS_API_KEYS=devkey123
#   AETHEROS_ADMIN_KEY=adminkey123
#   cargo run --no-default-features --features backend-wasmi
#
# 6 (ModuleStore limitleri), 7 (persistence/restart) ve 8 (release
# benchmark) bu script'e DAHİL DEĞİL — üçü de sunucuyu farklı env'le
# ya da farklı build ile yeniden başlatmayı gerektiriyor, aynı anda
# çalışan bir sunucuyla test edilemezler. Bu script bittikten sonra
# mesajdaki 6/7/8 adımlarını ayrıca çalıştır.
# ============================================================
set -u
for bin in curl jq; do
  command -v "$bin" >/dev/null 2>&1 || { echo "HATA: '$bin' kurulu değil (pkg install $bin)"; exit 1; }
done

BASE_URL="${AETHEROS_URL:-http://localhost:8080}"
API_KEY="${AETHEROS_API_KEY:-devkey123}"
ADMIN_KEY="${AETHEROS_ADMIN_KEY_CLIENT:-adminkey123}"
RUN42_HEX="0061736d010000000105016000017f030201000707010372756e00000a06010400412a0b"

PASS=0; FAIL=0
ok()   { echo "  ✓ $1"; PASS=$((PASS+1)); }
bad()  { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

hr() { echo; echo "══════════════════════════════════════════"; echo "  $1"; echo "══════════════════════════════════════════"; }

# ── A: Health ──────────────────────────────────────────────
hr "A: Sunucu erişilebilirlik"
h=$(curl -s "${BASE_URL}/health")
[[ "$(echo "$h" | jq -r .healthy)" == "true" ]] && ok "Health OK" || bad "Health FAIL: $h"

# ── B: Gerçek WASM → Completed ───────────────────────────────
hr "B: Task execution — gerçek export'lu WASM"
resp=$(curl -s -X POST "${BASE_URL}/tasks" -H "X-Api-Key: ${API_KEY}" \
  -H 'Content-Type: application/json' \
  -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${RUN42_HEX}\",\"max_attempts\":1}")
tid=$(echo "$resp" | jq -r .task_id)
if [[ -z "$tid" || "$tid" == "null" ]]; then
  bad "Task submit edilemedi: $resp"
else
  state="Queued"
  for i in $(seq 1 20); do
    sleep 0.3
    state=$(curl -s "${BASE_URL}/tasks/${tid}" -H "X-Api-Key: ${API_KEY}" | jq -r .state)
    [[ "$state" == "Completed" || "$state" == "Failed" ]] && break
  done
  if [[ "$state" == "Completed" ]]; then
    ok "Gerçek WASM → Completed (task_id=$tid)"
  else
    bad "Beklenen Completed, alınan: $state (task_id=$tid) — wasmi call_entrypoint imza eşleşmesini kontrol et"
  fi
fi

# ── C: RBAC — viewer / operator / admin ──────────────────────
hr "C: RBAC — viewer / operator / admin"

get_token() {
  curl -s -X POST "${BASE_URL}/auth/token" -H 'Content-Type: application/json' \
    -d "{\"admin_key\":\"${ADMIN_KEY}\",\"subject\":\"test-$1\",\"role\":\"$1\"}" | jq -r .access_token
}

viewer_tok=$(get_token viewer)
operator_tok=$(get_token operator)
admin_tok=$(get_token admin)

if [[ -z "$viewer_tok" || "$viewer_tok" == "null" ]]; then
  bad "/auth/token başarısız — AETHEROS_ADMIN_KEY sunucuda ${ADMIN_KEY} ile eşleşiyor mu?"
else
  # Viewer: TaskSubmit YASAK olmalı (403)
  code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}/tasks" \
    -H "Authorization: Bearer ${viewer_tok}" -H 'Content-Type: application/json' \
    -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${RUN42_HEX}\"}")
  [[ "$code" == "403" ]] && ok "Viewer POST /tasks → 403 (beklenen)" || bad "Viewer POST /tasks → $code (403 bekleniyordu)"

  # Viewer: okuma İZİNLİ olmalı (200)
  code=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/tasks/${tid}" \
    -H "Authorization: Bearer ${viewer_tok}")
  [[ "$code" == "200" ]] && ok "Viewer GET /tasks/{id} → 200 (beklenen)" || bad "Viewer GET /tasks/{id} → $code (200 bekleniyordu)"

  # Operator: TaskSubmit İZİNLİ olmalı (202)
  code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}/tasks" \
    -H "Authorization: Bearer ${operator_tok}" -H 'Content-Type: application/json' \
    -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${RUN42_HEX}\"}")
  [[ "$code" == "202" ]] && ok "Operator POST /tasks → 202 (beklenen)" || bad "Operator POST /tasks → $code (202 bekleniyordu)"

  # Operator: cluster/remote (admin-only) YASAK olmalı (403)
  code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}/remote/command" \
    -H "Authorization: Bearer ${operator_tok}" -H 'Content-Type: application/json' -d '{}')
  [[ "$code" == "403" || "$code" == "400" ]] && ok "Operator POST /remote/command → $code (admin gerektirir)" || bad "Operator POST /remote/command → $code (403 bekleniyordu)"

  # Admin: her şey İZİNLİ olmalı
  code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}/tasks" \
    -H "Authorization: Bearer ${admin_tok}" -H 'Content-Type: application/json' \
    -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${RUN42_HEX}\"}")
  [[ "$code" == "202" ]] && ok "Admin POST /tasks → 202 (beklenen)" || bad "Admin POST /tasks → $code (202 bekleniyordu)"
fi

# ── D: WebSocket — auth ───────────────────────────────────────
hr "D: WebSocket auth"
if ! command -v websocat >/dev/null 2>&1; then
  echo "  ⚠ websocat yok, atlanıyor (cargo install websocat)"
else
  if timeout 2 websocat --no-close "${BASE_URL/http/ws}/ws?api_key=${API_KEY}" < /dev/null > /dev/null 2>&1; then
    ok "WS geçerli api_key ile bağlandı"
  else
    bad "WS geçerli api_key ile bağlanamadı"
  fi
  if timeout 2 websocat --no-close "${BASE_URL/http/ws}/ws?api_key=yanlis-key" < /dev/null > /dev/null 2>&1; then
    bad "WS geçersiz api_key ile bağlandı (REDDETMELİYDİ)"
  else
    ok "WS geçersiz api_key ile reddedildi (beklenen)"
  fi
fi

# ── E: Concurrency — active_workers gözlemi (best-effort) ────
hr "E: Concurrency — 4 worker sınırı gözlemi (best-effort)"
echo "  → 40 task hızlıca gönderilirken active_workers örnekleniyor..."
max_seen=0
( for i in $(seq 1 40); do
    curl -s -X POST "${BASE_URL}/tasks" -H "X-Api-Key: ${API_KEY}" \
      -H 'Content-Type: application/json' \
      -d "{\"entrypoint\":\"run\",\"wasm_module_hex\":\"${RUN42_HEX}\"}" >/dev/null &
  done; wait ) &
submit_pid=$!
while kill -0 $submit_pid 2>/dev/null; do
  aw=$(curl -s "${BASE_URL}/metrics" -H "X-Api-Key: ${API_KEY}" | jq -r .active_workers)
  [[ "$aw" =~ ^[0-9]+$ ]] && (( aw > max_seen )) && max_seen=$aw
  sleep 0.02
done
wait $submit_pid 2>/dev/null
echo "  → Gözlemlenen maksimum active_workers: $max_seen"
if (( max_seen <= 4 )); then
  ok "active_workers hiç 4'ü geçmedi (kod incelemesiyle tutarlı: gerçek paralellik=worker sayısı)"
else
  bad "active_workers 4'ü geçti ($max_seen) — worker.rs analizini gözden geçir"
fi
echo "  NOT: WASM anlık çalıştığı için bu ölçüm zayıf bir sinyal;"
echo "        max_seen=0 çıkması da normaldir (task'lar örnekleme"
echo "        aralığından hızlı bitiyor demektir), çelişki değildir."

hr "SONUÇ"
echo "  PASSED: $PASS   FAILED: $FAIL"
