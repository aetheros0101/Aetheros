/**
 * @aetheros/sdk — TypeScript SDK for AetherOS Runtime
 *
 * Usage (Node.js / Browser / Deno):
 *
 * ```typescript
 * import { AetherClient } from "@aetheros/sdk";
 *
 * const client = new AetherClient("http://localhost:8080", {
 *   apiKey: process.env.AETHEROS_API_KEY,
 * });
 *
 * // Health check
 * const health = await client.health();
 *
 * // Task gönder
 * const { taskId } = await client.submitTask({
 *   entrypoint: "main",
 *   wasmHex: "0061736d...",
 *   priority: "high",
 * });
 *
 * // Tamamlanana kadar bekle
 * const result = await client.waitForCompletion(taskId);
 *
 * // Agent başlat
 * const { executionId } = await client.startAgent({
 *   objective: "Analyze and summarize the dataset",
 * });
 * ```
 */

// ── Types ──────────────────────────────────────────────────

export type Priority = "critical" | "high" | "normal" | "low";
export type TaskState =
  | "Created"
  | "Queued"
  | "Executing"
  | "Completed"
  | "Failed"
  | "Cancelled"
  | "Retrying";

export interface HealthResponse {
  healthy: boolean;
  version: string;
  timestamp: string;
}

export interface SubmitTaskRequest {
  entrypoint: string;
  wasmHex: string;
  priority?: Priority;
  timeoutMs?: number;
  maxAttempts?: number;
}

export interface SubmitTaskResponse {
  taskId: string;
  status: string;
}

export interface TaskStateResponse {
  taskId: string;
  state: TaskState;
  attempts: number;
  createdAt: string;
  updatedAt: string;
}

export interface StartAgentRequest {
  objective: string;
  maxSteps?: number;
  maxTokens?: number;
}

export interface StartAgentResponse {
  executionId: string;
  status: string;
}

export interface SubmitWorkflowResponse {
  workflowId: string;
  status: string;
  name: string;
}

export interface MetricsResponse {
  activeWorkers: number;
  queuedTasks: number;
  completedTasks: number;
  failedTasks: number;
  retriedTasks: number;
  timestamp: string;
}

export interface ClientOptions {
  /** Bearer JWT token */
  jwtToken?: string;
  /** API key (X-Api-Key header) */
  apiKey?: string;
  /** Request timeout ms (default: 30_000) */
  timeoutMs?: number;
}

// ── Error ──────────────────────────────────────────────────

export class AetherError extends Error {
  constructor(
    public readonly statusCode: number,
    public readonly message: string,
  ) {
    super(`AetherOS API error ${statusCode}: ${message}`);
    this.name = "AetherError";
  }
}

// ── Client ─────────────────────────────────────────────────

export class AetherClient {
  private readonly endpoint: string;
  private readonly headers: Record<string, string>;
  private readonly timeoutMs: number;

  constructor(endpoint: string, options: ClientOptions = {}) {
    this.endpoint = endpoint.replace(/\/$/, "");
    this.timeoutMs = options.timeoutMs ?? 30_000;

    this.headers = {
      "Content-Type": "application/json",
      Accept: "application/json",
    };

    if (options.jwtToken) {
      this.headers["Authorization"] = `Bearer ${options.jwtToken}`;
    } else if (options.apiKey) {
      this.headers["X-Api-Key"] = options.apiKey;
    }
  }

  // ── Health ───────────────────────────────────────────────

  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>("/health");
  }

  // ── Tasks ────────────────────────────────────────────────

  async submitTask(
    req: SubmitTaskRequest,
  ): Promise<SubmitTaskResponse> {
    const response = await this.post<{
      task_id: string;
      status: string;
    }>("/tasks", {
      entrypoint: req.entrypoint,
      wasm_module_hex: req.wasmHex,
      priority: req.priority ?? "normal",
      timeout_ms: req.timeoutMs ?? 30_000,
      max_attempts: req.maxAttempts ?? 3,
    });

    return {
      taskId: response.task_id,
      status: response.status,
    };
  }

  async taskState(taskId: string): Promise<TaskStateResponse> {
    const r = await this.get<{
      task_id: string;
      state: TaskState;
      attempts: number;
      created_at: string;
      updated_at: string;
    }>(`/tasks/${taskId}`);

    return {
      taskId: r.task_id,
      state: r.state,
      attempts: r.attempts,
      createdAt: r.created_at,
      updatedAt: r.updated_at,
    };
  }

  async cancelTask(taskId: string): Promise<void> {
    await this.post(`/tasks/${taskId}/cancel`, {});
  }

  /**
   * Task terminal state'e ulaşana kadar polling yap.
   * Terminal: Completed | Failed | Cancelled
   */
  async waitForCompletion(
    taskId: string,
    options: {
      pollIntervalMs?: number;
      timeoutMs?: number;
    } = {},
  ): Promise<TaskStateResponse> {
    const poll = options.pollIntervalMs ?? 500;
    const timeout = options.timeoutMs ?? 60_000;
    const terminal = new Set(["Completed", "Failed", "Cancelled"]);

    const deadline = Date.now() + timeout;

    while (Date.now() < deadline) {
      const state = await this.taskState(taskId);
      if (terminal.has(state.state)) return state;
      await sleep(poll);
    }

    throw new Error(
      `Task ${taskId} did not complete within ${timeout}ms`,
    );
  }

  // ── Agents ───────────────────────────────────────────────

  async startAgent(
    req: StartAgentRequest,
  ): Promise<StartAgentResponse> {
    const r = await this.post<{
      execution_id: string;
      status: string;
    }>("/agents", {
      objective: req.objective,
      max_steps: req.maxSteps ?? 10,
      max_tokens: req.maxTokens ?? 4096,
    });

    return {
      executionId: r.execution_id,
      status: r.status,
    };
  }

  // ── Workflows ─────────────────────────────────────────────

  async submitWorkflow(
    name: string,
  ): Promise<SubmitWorkflowResponse> {
    const r = await this.post<{
      workflow_id: string;
      status: string;
      name: string;
    }>("/workflows", { name });

    return {
      workflowId: r.workflow_id,
      status: r.status,
      name: r.name,
    };
  }

  // ── Metrics ───────────────────────────────────────────────

  async metrics(): Promise<MetricsResponse> {
    const r = await this.get<{
      active_workers: number;
      queued_tasks: number;
      completed_tasks: number;
      failed_tasks: number;
      retried_tasks: number;
      timestamp: string;
    }>("/metrics");

    return {
      activeWorkers: r.active_workers,
      queuedTasks: r.queued_tasks,
      completedTasks: r.completed_tasks,
      failedTasks: r.failed_tasks,
      retriedTasks: r.retried_tasks,
      timestamp: r.timestamp,
    };
  }

  // ── WebSocket Event Stream ────────────────────────────────

  /**
   * WebSocket event stream'e bağlan.
   * Her event callback ile iletilir.
   * Bağlantı kapatmak için döndürülen fonksiyonu çağır.
   */
  subscribe(
    onEvent: (event: { type: string; event: string }) => void,
    onError?: (err: Event) => void,
  ): () => void {
    const wsUrl = this.endpoint
      .replace(/^http/, "ws")
      .replace(/^https/, "wss");

    const ws = new WebSocket(`${wsUrl}/ws`);

    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data as string);
        onEvent(msg);
      } catch {}
    };

    if (onError) ws.onerror = onError;

    return () => ws.close();
  }

  // ── Utilities ─────────────────────────────────────────────

  /** WASM dosyasını hex string'e çevir (Node.js) */
  static wasmToHex(buffer: ArrayBuffer | Uint8Array): string {
    const bytes =
      buffer instanceof Uint8Array
        ? buffer
        : new Uint8Array(buffer);
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  }

  // ── HTTP ──────────────────────────────────────────────────

  private async get<T>(path: string): Promise<T> {
    const resp = await this.fetch(path, { method: "GET" });
    return this.parse<T>(resp);
  }

  private async post<T>(
    path: string,
    body: unknown,
  ): Promise<T> {
    const resp = await this.fetch(path, {
      method: "POST",
      body: JSON.stringify(body),
    });
    return this.parse<T>(resp);
  }

  private async fetch(
    path: string,
    init: RequestInit,
  ): Promise<Response> {
    const controller = new AbortController();
    const timer = setTimeout(
      () => controller.abort(),
      this.timeoutMs,
    );

    try {
      return await globalThis.fetch(
        `${this.endpoint}${path}`,
        {
          ...init,
          headers: this.headers,
          signal: controller.signal,
        },
      );
    } finally {
      clearTimeout(timer);
    }
  }

  private async parse<T>(resp: Response): Promise<T> {
    if (!resp.ok) {
      let msg = resp.statusText;
      try {
        const body = await resp.json() as { error?: string };
        msg = body.error ?? msg;
      } catch {}
      throw new AetherError(resp.status, msg);
    }
    return resp.json() as Promise<T>;
  }
}

// ── Helpers ───────────────────────────────────────────────

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// ── Default export ────────────────────────────────────────

export default AetherClient;
