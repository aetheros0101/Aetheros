"""
aetheros — Python SDK for AetherOS Runtime

Async-first client using httpx.

Usage:
    import asyncio
    from aetheros import AetherClient

    async def main():
        async with AetherClient("http://localhost:8080") as client:
            # Health check
            health = await client.health()
            print(health)

            # Task gönder
            result = await client.submit_task(
                entrypoint="main",
                wasm_hex="0061736d...",
                priority="high",
                timeout_ms=10_000,
            )
            task_id = result["task_id"]

            # State sorgula
            state = await client.task_state(task_id)
            print(state)

            # Agent başlat
            agent = await client.start_agent(
                objective="Analyze the data and summarize",
                max_steps=5,
                max_tokens=2048,
            )

            # Workflow gönder
            wf = await client.submit_workflow("my-pipeline")

    asyncio.run(main())
"""

from __future__ import annotations

import asyncio
import hashlib
from typing import Any, Dict, Optional
from uuid import UUID

try:
    import httpx
except ImportError:
    raise ImportError(
        "aetheros SDK requires httpx. "
        "Install it with: pip install httpx"
    )


class AetherError(Exception):
    """AetherOS API hatası."""

    def __init__(
        self,
        status_code: int,
        message: str,
    ) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(
            f"AetherOS API error {status_code}: {message}"
        )


class AetherClient:
    """
    AetherOS async REST client.

    Context manager olarak kullanın:
        async with AetherClient("http://localhost:8080") as c:
            await c.health()

    Veya manuel yönetin:
        client = AetherClient("http://localhost:8080")
        await client.health()
        await client.close()
    """

    def __init__(
        self,
        endpoint: str,
        *,
        api_key: Optional[str] = None,
        jwt_token: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        self._endpoint = endpoint.rstrip("/")
        self._timeout = timeout

        # Auth header'ları
        headers: Dict[str, str] = {
            "Content-Type": "application/json",
            "Accept": "application/json",
        }
        if jwt_token:
            headers["Authorization"] = f"Bearer {jwt_token}"
        elif api_key:
            headers["X-Api-Key"] = api_key

        self._client = httpx.AsyncClient(
            headers=headers,
            timeout=timeout,
        )

    async def __aenter__(self) -> "AetherClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.close()

    async def close(self) -> None:
        await self._client.aclose()

    # ── Health ────────────────────────────────────────────

    async def health(self) -> Dict[str, Any]:
        """GET /health → sistem durumu."""
        return await self._get("/health")

    # ── Tasks ─────────────────────────────────────────────

    async def submit_task(
        self,
        entrypoint: str,
        wasm_hex: str,
        *,
        priority: str = "normal",
        timeout_ms: int = 30_000,
        max_attempts: int = 3,
    ) -> Dict[str, Any]:
        """
        POST /tasks → task kuyruğuna ekle.

        Args:
            entrypoint: WASM modülündeki fonksiyon adı
            wasm_hex: WASM binary hex string
            priority: "critical" | "high" | "normal" | "low"
            timeout_ms: task timeout (ms)
            max_attempts: max retry sayısı
        
        Returns:
            {"task_id": str, "status": "queued"}
        """
        return await self._post(
            "/tasks",
            {
                "entrypoint": entrypoint,
                "wasm_module_hex": wasm_hex,
                "priority": priority,
                "timeout_ms": timeout_ms,
                "max_attempts": max_attempts,
            },
        )

    async def submit_task_from_file(
        self,
        wasm_path: str,
        entrypoint: str = "main",
        **kwargs: Any,
    ) -> Dict[str, Any]:
        """
        WASM dosyasından task gönder.
        Dosyayı okuyup hex'e çevirir.
        """
        with open(wasm_path, "rb") as f:
            wasm_bytes = f.read()
        wasm_hex = wasm_bytes.hex()
        return await self.submit_task(
            entrypoint, wasm_hex, **kwargs
        )

    async def task_state(
        self,
        task_id: str | UUID,
    ) -> Dict[str, Any]:
        """
        GET /tasks/{id} → task durumu.

        Returns:
            {"task_id": str, "state": str, "attempts": int, ...}
        """
        return await self._get(f"/tasks/{task_id}")

    async def cancel_task(
        self,
        task_id: str | UUID,
    ) -> Dict[str, Any]:
        """POST /tasks/{id}/cancel → task'ı iptal et."""
        return await self._post(
            f"/tasks/{task_id}/cancel", {}
        )

    async def wait_for_completion(
        self,
        task_id: str | UUID,
        *,
        poll_interval: float = 0.5,
        timeout: float = 60.0,
    ) -> Dict[str, Any]:
        """
        Task tamamlanana kadar polling yap.
        
        Terminal state'ler: Completed, Failed, Cancelled.
        
        Raises:
            asyncio.TimeoutError: timeout dolunca
            AetherError: API hatası
        """
        terminal = {"Completed", "Failed", "Cancelled"}
        elapsed = 0.0

        while elapsed < timeout:
            state = await self.task_state(task_id)
            if state.get("state") in terminal:
                return state
            await asyncio.sleep(poll_interval)
            elapsed += poll_interval

        raise asyncio.TimeoutError(
            f"Task {task_id} did not complete within {timeout}s"
        )

    # ── Agents ────────────────────────────────────────────

    async def start_agent(
        self,
        objective: str,
        *,
        max_steps: int = 10,
        max_tokens: int = 4096,
    ) -> Dict[str, Any]:
        """
        POST /agents → agent başlat.

        Returns:
            {"execution_id": str, "status": "started"}
        """
        return await self._post(
            "/agents",
            {
                "objective": objective,
                "max_steps": max_steps,
                "max_tokens": max_tokens,
            },
        )

    # ── Workflows ─────────────────────────────────────────

    async def submit_workflow(
        self,
        name: str,
    ) -> Dict[str, Any]:
        """
        POST /workflows → workflow gönder.

        Returns:
            {"workflow_id": str, "status": "accepted"}
        """
        return await self._post(
            "/workflows",
            {"name": name},
        )

    # ── Metrics ───────────────────────────────────────────

    async def metrics(self) -> Dict[str, Any]:
        """GET /metrics → sistem metrikleri."""
        return await self._get("/metrics")

    # ── Utilities ─────────────────────────────────────────

    @staticmethod
    def wasm_hash(wasm_bytes: bytes) -> str:
        """WASM binary SHA-256 hash'ini hex olarak döndür."""
        return hashlib.sha256(wasm_bytes).hexdigest()

    # ── HTTP ──────────────────────────────────────────────

    async def _get(self, path: str) -> Dict[str, Any]:
        resp = await self._client.get(
            f"{self._endpoint}{path}"
        )
        return self._parse(resp)

    async def _post(
        self,
        path: str,
        body: Dict[str, Any],
    ) -> Dict[str, Any]:
        resp = await self._client.post(
            f"{self._endpoint}{path}",
            json=body,
        )
        return self._parse(resp)

    def _parse(
        self,
        resp: httpx.Response,
    ) -> Dict[str, Any]:
        if not resp.is_success:
            try:
                msg = resp.json().get("error", resp.text)
            except Exception:
                msg = resp.text
            raise AetherError(resp.status_code, msg)
        return resp.json()
