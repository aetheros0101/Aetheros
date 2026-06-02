// ============================================================
// benches/runtime_bench.rs  (v2)
//
// Optimizasyon #2: JSON vs MessagePack karşılaştırması eklendi
// ============================================================

use criterion::{
    criterion_group,
    criterion_main,
    BenchmarkId,
    Criterion,
    Throughput,
};
use std::hint::black_box;

use chrono::Utc;
use uuid::Uuid;
use rmp_serde;

use aetheros::task::priority::TaskPriority;
use aetheros::task::retry::RetryPolicy;
use aetheros::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use aetheros::types::ids::TaskId;

fn make_task(priority: TaskPriority) -> TaskDefinition {
    TaskDefinition {
        id: TaskId(Uuid::new_v4()),
        parent: None,
        orchestration: None,
        priority,
        deadline: None,
        timeout_ms: 5000,
        retry_policy: RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            jitter: false,
        },
        metadata: TaskMetadata {
            labels: Default::default(),
        },
        wasm_module_hash: [0xABu8; 32], // dummy hash
        entrypoint: "main".to_string(),
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ── Priority Queue ────────────────────────────────────────

fn bench_queue(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("priority_queue");

    group.throughput(Throughput::Elements(1));
    group.bench_function("push_pop_normal", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = aetheros::task::queue::PriorityTaskQueue::new();
                let task = make_task(TaskPriority::Normal);
                q.push(black_box(task)).await.unwrap();
                q.pop().await.unwrap()
            })
        })
    });

    group.throughput(Throughput::Elements(100));
    group.bench_function("push_pop_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = aetheros::task::queue::PriorityTaskQueue::new();
                for _ in 0..100 {
                    q.push(make_task(TaskPriority::Normal))
                        .await.unwrap();
                }
                for _ in 0..100 {
                    q.pop().await.unwrap();
                }
            })
        })
    });

    group.finish();
}

// ── Priority Ordering ─────────────────────────────────────

fn bench_priority(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("priority_ordering");

    group.bench_function("mixed_4_priorities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = aetheros::task::queue::PriorityTaskQueue::new();
                q.push(make_task(TaskPriority::Low)).await.unwrap();
                q.push(make_task(TaskPriority::Critical)).await.unwrap();
                q.push(make_task(TaskPriority::Normal)).await.unwrap();
                q.push(make_task(TaskPriority::High)).await.unwrap();
                let first = q.pop().await.unwrap();
                black_box(first.priority)
            })
        })
    });

    group.finish();
}

// ── Retry Delay ───────────────────────────────────────────

fn bench_retry_delay(c: &mut Criterion) {
    let policy = aetheros::task::retry::RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 100,
        max_delay_ms: 30_000,
        jitter: false,
    };

    let mut group = c.benchmark_group("retry_delay");

    for attempt in [1u32, 3, 5, 8] {
        group.bench_with_input(
            BenchmarkId::new("next_delay", attempt),
            &attempt,
            |b, &attempt| {
                b.iter(|| policy.next_delay(black_box(attempt)))
            },
        );
    }

    group.finish();
}

// ── Serde: JSON vs MessagePack ────────────────────────────

fn bench_serde(c: &mut Criterion) {
    let task = make_task(TaskPriority::High);

    // Serialize
    let json = serde_json::to_string(&task).unwrap();
    let msgpack = rmp_serde::to_vec_named(&task).unwrap();

    let mut group = c.benchmark_group("task_serde");

    // JSON
    group.bench_function("json_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&task)).unwrap())
    });

    group.bench_function("json_deserialize", |b| {
        b.iter(|| {
            serde_json::from_str::<TaskDefinition>(
                black_box(&json),
            )
            .unwrap()
        })
    });

    // MessagePack
    group.bench_function("msgpack_serialize", |b| {
        b.iter(|| rmp_serde::to_vec_named(black_box(&task)).unwrap())
    });

    group.bench_function("msgpack_deserialize", |b| {
        b.iter(|| {
            rmp_serde::from_slice::<TaskDefinition>(
                black_box(&msgpack),
            )
            .unwrap()
        })
    });

    group.finish();

    // Boyut karşılaştırması — tek seferlik
    println!(
        "\n📦 Payload boyutu: JSON={} bytes, MsgPack={} bytes ({:.0}% küçük)",
        json.len(),
        msgpack.len(),
        (1.0 - msgpack.len() as f64 / json.len() as f64) * 100.0
    );
}

criterion_group!(
    benches,
    bench_queue,
    bench_priority,
    bench_retry_delay,
    bench_serde,
);
criterion_main!(benches);
