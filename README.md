# Async Job Orchestrator (Core Immediate Execution)

**Version 0.1**  
**Scope:** Single-tenant • Immediate execution • In-memory state  
_No scheduling / authentication yet._

---

## 1️⃣ Goal
Deliver a minimal single-tenant orchestrator that accepts job submissions over HTTP, executes jobs asynchronously and immediately (no scheduling), tracks state, logs, and results in memory, and exposes job status and basic metrics.  
_No persistence, multi-tenancy, or authentication in this phase._

---

## 2️⃣ Core Concepts & States

**Job**
- Immutable metadata (`id`, `type`, `payload`)
- Mutable runtime fields (`state`, `result`, `log`, `created_at`, `started_at`, `finished_at`)

**State Transitions**

```
QUEUED → RUNNING → (SUCCEEDED | FAILED)
```

---

## 3️⃣ Functional Requirements

### 3.1 Job Submission
**Endpoint:** `POST /jobs`

**Request Body**
```json
{
  "type": "echo",
  "payload": { "message": "hello world" }
}
```

**Behavior**
- Assign a globally unique `job_id` (ULID or UUIDv7).  
- Immediately enqueue for execution. If at capacity, respond `202 Accepted` with `state = QUEUED`.  
- Response contains `job_id` and initial `state`.

---

### 3.2 Job Execution
- Maintain an execution pool with configurable max concurrency (default = 4).  
- Dispatch next `QUEUED` job when slot is free; transition to `RUNNING`.  
- Supported job types (initial set):
  - `echo` → return payload  
  - `sleep` → payload `{"ms": <number>}`; sleep for given duration, then return `"ok"`.  
- On success: record `result` (stringified JSON) and mark `SUCCEEDED`.  
- On panic/error: record error string and mark `FAILED`.  
- Capture per-job log (append-only string buffer ≤ 64 KB).

---

### 3.3 Job Status & Result Query
**Endpoint:** `GET /jobs/{job_id}`  

Returns full job record:
```json
{
  "job_id": "...",
  "type": "echo",
  "state": "SUCCEEDED",
  "created_at": "...",
  "started_at": "...",
  "finished_at": "...",
  "result": "...",
  "log": "..."
}
```

If `job_id` unknown → `404 Not Found`.

---

### 3.4 Metrics Endpoint
**Endpoint:** `GET /metrics`  

**Response**
```json
{
  "total_submitted": 42,
  "running": 3,
  "queued": 5,
  "succeeded": 30,
  "failed": 4,
  "avg_duration_ms": 512
}
```
Counters reset on restart.

---

## 4️⃣ Non-Functional Requirements

| Aspect | Requirement |
|:--|:--|
| Concurrency | Configurable max concurrent jobs (default 4). |
| Throughput | ≈ 100 req/s submission target on local machine. |
| Latency | p95 job start < 100 ms under light load. |
| Memory | Limit in-memory job history to 1000 entries (evict oldest SUCCEEDED/FAILED). |
| Error Handling | Structured JSON errors (400/404/500). No panics on invalid input. |
| Logging | Log method, path, status, latency per API call. |
| Config | Env vars for runtime params (`PORT`, `MAX_CONCURRENCY`, `MAX_JOBS`). |

---

## 5️⃣ Error Model (Client-Visible)

| HTTP Code | Meaning |
|:--|:--|
| 400 | Invalid JSON or missing field |
| 404 | Job not found |
| 413 | Payload too large (> 1 MB) |
| 429 | Too many queued jobs (capacity limit) |
| 500 | Internal error (panic in handler) |

---

## 6️⃣ Operational Requirements
- Graceful shutdown on `SIGINT/SIGTERM`: stop new submissions, let running jobs finish.  
- Log to stdout (text or JSON), including job id and type.  
- Configurable port (`PORT`, default 8080).  

---

## 7️⃣ Acceptance Criteria (Sanity Tests)
1. Submitting a job returns `201`/`202` with unique `job_id` and `state = QUEUED`.  
2. `GET /jobs/{id}` shows transition: `QUEUED → RUNNING → SUCCEEDED`.  
3. Concurrent `sleep` jobs respect global max concurrency.  
4. Failed jobs show `state = FAILED` with non-empty log.  
5. `/metrics` accurately reflects job counts and latencies.  
6. Graceful shutdown handles SIGINT cleanly (no crash, no lost jobs).  
