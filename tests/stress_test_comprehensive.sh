#!/bin/bash

# ============================================================
# AetherOS Comprehensive Stress Test
# 
# Uyumlu: Termux, Linux, macOS
# Test Senaryoları:
#   - Memory leak detection
#   - Task completion tracking
#   - Worker efficiency
#   - Event bus performance
#   - Persistence under load
#   - Recovery scenarios
# ============================================================

set -e

# ─────────────────────────────────────────────────────────
# CONFIG
# ─────────────────────────────────────────────────────────

API_URL="${AETHEROS_API:-http://localhost:8080}"
RESULTS_DIR="${RESULTS_DIR:-.}"
DB_PATH="${AETHEROS_DB_PATH:-./aetheros_test.db}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="${RESULTS_DIR}/stress_test_${TIMESTAMP}.json"

# Test parameters
TASK_BATCH_SIZE=100
SEQUENTIAL_TASKS=500
PARALLEL_TASKS=50
STRESS_DURATION=60
MEMORY_SAMPLE_INTERVAL=2

# ─────────────────────────────────────────────────────────
# UTILITIES
# ─────────────────────────────────────────────────────────

# Color output (Termux compatible)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Memory snapshot (Termux: ps, Linux: /proc/self/status)
get_memory_mb() {
    if [ -f /proc/self/status ]; then
        # Linux
        grep VmRSS /proc/self/status | awk '{print int($2/1024)}'
    else
        # Termux/macOS fallback
        ps aux | grep "aetheros" | grep -v grep | awk '{print int($6/1024)}'
    fi
}

# Get metrics from API
get_metrics() {
    curl -s "${API_URL}/metrics" 2>/dev/null || echo '{"error":"unreachable"}'
}

# ─────────────────────────────────────────────────────────
# TEST: Health Check
# ─────────────────────────────────────────────────────────

test_health() {
    log_info "TEST 1: Health Check"
    
    response=$(curl -s -w "\n%{http_code}" "${API_URL}/health")
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" = "200" ]; then
        log_success "API healthy"
        echo "$body" | jq . 2>/dev/null || echo "$body"
        return 0
    else
        log_error "API unhealthy (HTTP $http_code)"
        return 1
    fi
}

# ─────────────────────────────────────────────────────────
# TEST: Sequential Task Submission
# ─────────────────────────────────────────────────────────

test_sequential_tasks() {
    log_info "TEST 2: Sequential Task Submission ($SEQUENTIAL_TASKS tasks)"
    
    local start_time=$(date +%s%N)
    local start_mem=$(get_memory_mb)
    local success_count=0
    local fail_count=0
    
    for i in $(seq 1 $SEQUENTIAL_TASKS); do
        task_id=$(uuidgen 2>/dev/null || echo "task-$i-$(date +%s%N)")
        
        response=$(curl -s -X POST "${API_URL}/tasks" \
            -H "Content-Type: application/json" \
            -d "{
                \"id\": \"$task_id\",
                \"entrypoint\": \"_start\",
                \"timeout_ms\": 5000,
                \"priority\": 1
            }" 2>/dev/null)
        
        if echo "$response" | grep -q "id"; then
            ((success_count++))
        else
            ((fail_count++))
        fi
        
        # Progress
        if [ $((i % 100)) -eq 0 ]; then
            current_mem=$(get_memory_mb)
            echo -e "  → $i/$SEQUENTIAL_TASKS submitted | RAM: ${current_mem}MB"
        fi
    done
    
    local end_time=$(date +%s%N)
    local end_mem=$(get_memory_mb)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    local mem_delta=$((end_mem - start_mem))
    
    log_success "Sequential tasks submitted"
    echo "  Success: $success_count | Failed: $fail_count"
    echo "  Duration: ${duration_ms}ms"
    echo "  Memory delta: ${mem_delta}MB (+$((mem_delta))MB)"
    echo "  Throughput: $(( SEQUENTIAL_TASKS * 1000 / (duration_ms + 1) )) tasks/sec"
    
    echo "sequential_tasks=$success_count" >> "$RESULTS_FILE"
    echo "sequential_duration_ms=$duration_ms" >> "$RESULTS_FILE"
}

# ─────────────────────────────────────────────────────────
# TEST: Parallel Task Burst
# ─────────────────────────────────────────────────────────

test_parallel_burst() {
    log_info "TEST 3: Parallel Task Burst ($PARALLEL_TASKS concurrent)"
    
    local start_time=$(date +%s%N)
    local start_mem=$(get_memory_mb)
    
    for i in $(seq 1 $PARALLEL_TASKS); do
        task_id=$(uuidgen 2>/dev/null || echo "parallel-$i-$(date +%s%N)")
        
        (curl -s -X POST "${API_URL}/tasks" \
            -H "Content-Type: application/json" \
            -d "{
                \"id\": \"$task_id\",
                \"entrypoint\": \"_start\",
                \"timeout_ms\": 3000,
                \"priority\": 2
            }" > /dev/null 2>&1) &
    done
    
    wait  # Wait for all background jobs
    
    local end_time=$(date +%s%N)
    local end_mem=$(get_memory_mb)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    log_success "Parallel burst completed"
    echo "  Duration: ${duration_ms}ms"
    echo "  Memory: ${start_mem}MB → ${end_mem}MB"
    echo "  Throughput: $(( PARALLEL_TASKS * 1000 / (duration_ms + 1) )) tasks/sec"
    
    echo "parallel_burst_duration_ms=$duration_ms" >> "$RESULTS_FILE"
}

# ─────────────────────────────────────────────────────────
# TEST: Memory Leak Detection
# ─────────────────────────────────────────────────────────

test_memory_leak() {
    log_info "TEST 4: Memory Leak Detection (${STRESS_DURATION}s)"
    
    local start_mem=$(get_memory_mb)
    local peak_mem=$start_mem
    local prev_mem=$start_mem
    local mem_samples=0
    local stable_count=0
    
    echo "  Start RAM: ${start_mem}MB"
    
    for i in $(seq 1 $((STRESS_DURATION / MEMORY_SAMPLE_INTERVAL))); do
        sleep $MEMORY_SAMPLE_INTERVAL
        
        local current_mem=$(get_memory_mb)
        local mem_delta=$((current_mem - prev_mem))
        
        echo "  [$((i * MEMORY_SAMPLE_INTERVAL))s] RAM: ${current_mem}MB (Δ${mem_delta}MB)"
        
        if [ $current_mem -gt $peak_mem ]; then
            peak_mem=$current_mem
        fi
        
        # Stability check: if memory stable for 5+ samples, consider it healthy
        if [ $mem_delta -le 5 ]; then
            ((stable_count++))
        else
            stable_count=0
        fi
        
        prev_mem=$current_mem
        ((mem_samples++))
    done
    
    local final_mem=$(get_memory_mb)
    local total_delta=$((final_mem - start_mem))
    local growth_rate=$(( (total_delta * 100) / (start_mem + 1) ))
    
    if [ $total_delta -lt 50 ]; then
        log_success "Memory stable (Δ${total_delta}MB, growth: ${growth_rate}%)"
    elif [ $total_delta -lt 200 ]; then
        log_warn "Moderate memory growth (Δ${total_delta}MB, growth: ${growth_rate}%)"
    else
        log_error "Excessive memory growth (Δ${total_delta}MB, growth: ${growth_rate}%)"
    fi
    
    echo "  Peak RAM: ${peak_mem}MB"
    echo "  Stability: ${stable_count}/${mem_samples} samples"
    
    echo "memory_start_mb=$start_mem" >> "$RESULTS_FILE"
    echo "memory_final_mb=$final_mem" >> "$RESULTS_FILE"
    echo "memory_peak_mb=$peak_mem" >> "$RESULTS_FILE"
    echo "memory_delta_mb=$total_delta" >> "$RESULTS_FILE"
    echo "memory_growth_rate=$growth_rate" >> "$RESULTS_FILE"
}

# ─────────────────────────────────────────────────────────
# TEST: Metrics & Task Completion
# ─────────────────────────────────────────────────────────

test_metrics() {
    log_info "TEST 5: Metrics & Task Completion Tracking"
    
    local metrics=$(get_metrics)
    
    if echo "$metrics" | jq . > /dev/null 2>&1; then
        local completed=$(echo "$metrics" | jq '.completed_tasks // 0')
        local failed=$(echo "$metrics" | jq '.failed_tasks // 0')
        local retried=$(echo "$metrics" | jq '.retried_tasks // 0')
        local queued=$(echo "$metrics" | jq '.queued_tasks // 0')
        local active=$(echo "$metrics" | jq '.active_workers // 0')
        
        log_success "Metrics retrieved"
        echo "  Completed: $completed"
        echo "  Failed: $failed"
        echo "  Retried: $retried"
        echo "  Queued: $queued"
        echo "  Active workers: $active"
        
        if [ "$completed" -eq 0 ]; then
            log_warn "No tasks completed! Check if tasks are actually running."
        fi
        
        echo "metrics_completed=$completed" >> "$RESULTS_FILE"
        echo "metrics_failed=$failed" >> "$RESULTS_FILE"
        echo "metrics_retried=$retried" >> "$RESULTS_FILE"
    else
        log_error "Failed to get metrics"
        echo "$metrics"
    fi
}

# ─────────────────────────────────────────────────────────
# TEST: Event Bus Stress
# ─────────────────────────────────────────────────────────

test_event_bus_stress() {
    log_info "TEST 6: Event Bus Stress ($TASK_BATCH_SIZE rapid tasks)"
    
    local start_time=$(date +%s%N)
    
    for i in $(seq 1 $TASK_BATCH_SIZE); do
        task_id=$(uuidgen 2>/dev/null || echo "eventbus-$i-$(date +%s%N)")
        
        (curl -s -X POST "${API_URL}/tasks" \
            -H "Content-Type: application/json" \
            -d "{
                \"id\": \"$task_id\",
                \"entrypoint\": \"_start\",
                \"timeout_ms\": 1000,
                \"priority\": 3
            }" > /dev/null 2>&1) &
    done
    
    wait
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    local throughput=$(( TASK_BATCH_SIZE * 1000 / (duration_ms + 1) ))
    
    log_success "Event bus stress test completed"
    echo "  Batch size: $TASK_BATCH_SIZE"
    echo "  Duration: ${duration_ms}ms"
    echo "  Throughput: $throughput events/sec"
    
    echo "eventbus_throughput=$throughput" >> "$RESULTS_FILE"
}

# ─────────────────────────────────────────────────────────
# TEST: Persistence Recovery
# ─────────────────────────────────────────────────────────

test_persistence() {
    log_info "TEST 7: Persistence & Recovery"
    
    # Check if DB exists and has tasks
    if [ -f "$DB_PATH" ]; then
        local db_size=$(ls -lh "$DB_PATH" | awk '{print $5}')
        log_success "Persistence DB: $db_size"
        echo "persistence_db_size=$db_size" >> "$RESULTS_FILE"
    else
        log_warn "No persistence DB found at $DB_PATH"
    fi
}

# ─────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────

main() {
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║    AetherOS Comprehensive Stress Test                      ║"
    echo "║    Timestamp: $TIMESTAMP                    ║"
    echo "║    API: $API_URL"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Initialize results file
    echo "timestamp=$TIMESTAMP" > "$RESULTS_FILE"
    echo "api_url=$API_URL" >> "$RESULTS_FILE"
    
    # Run tests
    test_health || { log_error "Health check failed, aborting"; exit 1; }
    echo ""
    
    test_sequential_tasks
    echo ""
    
    test_parallel_burst
    echo ""
    
    test_event_bus_stress
    echo ""
    
    # Wait for tasks to process
    log_info "Waiting for tasks to process (10s)..."
    sleep 10
    echo ""
    
    test_memory_leak
    echo ""
    
    test_metrics
    echo ""
    
    test_persistence
    echo ""
    
    # Summary
    echo "╔════════════════════════════════════════════════════════════╗"
    log_success "Stress test completed!"
    echo "║ Results saved to: $RESULTS_FILE"
    echo "╚════════════════════════════════════════════════════════════╝"
    
    # Print results
    echo ""
    log_info "Summary:"
    cat "$RESULTS_FILE"
}

# Signal handler
cleanup() {
    log_warn "Test interrupted!"
    exit 130
}

trap cleanup SIGINT SIGTERM

# Run
main "$@"
