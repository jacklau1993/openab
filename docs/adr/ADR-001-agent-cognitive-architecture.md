# ADR-001: Agent Cognitive Architecture Specification (ACAS)

- **Status**: Proposed
- **Spec Version**: 1.0.0
- **Date**: 2026-04-23
- **Author**: pahud.hsieh

## Context

OpenAB is a multi-bot, agent-agnostic, vendor-agnostic platform. It bridges multiple coding CLIs (Kiro, Claude Code, Codex, Gemini, Copilot, OpenCode, Cursor, etc.) into chat platforms like Discord and Slack, where multiple bots/agents coexist in the same chatroom.

When multiple agents share a chatroom, a critical challenge emerges: **how does each agent quickly establish its own cognitive system — identity, memory, and social relationships — so they can effectively communicate, coordinate, and collaborate?**

Today, each agent operates in isolation without a shared standard. This leads to:

1. No consistent persona — agents don't know "who they are" across sessions
2. No social awareness — agents don't know who else is in the room, what they're good at, or how to mention them
3. No persistent memory — knowledge is lost between sessions with no mechanism to accumulate and refine it over time

For OpenAB's multi-agent vision to work, we need a generic, platform-agnostic specification that any agent implementation can follow to bootstrap these cognitive capabilities — regardless of the underlying LLM or framework.

## Decision

We adopt a three-pillar cognitive architecture for agents:

1. **Self-Identity System** — defines who the agent is
2. **Social Awareness System** — defines who else exists and how to interact
3. **Knowledge System** — defines how the agent remembers, recalls, and refines knowledge

---

## 1. Self-Identity System

Every agent MUST maintain a self-identity definition that answers: **"Who am I?"**

### Required Identity Fields

```yaml
spec_version: "1.0.0"
identity:
  name: ""            # Agent's name (how it refers to itself)
  uid: ""             # Unique identifier (platform-specific, e.g. Discord UID)
  persona: ""         # One-line self-description
  personality: []     # List of personality traits
  tone: ""            # Communication style (e.g. humorous, formal, blunt)
  language: []        # Preferred languages, in order
  origin: ""          # Backstory or origin (optional)
  capabilities: []    # Supported tool versions (e.g. ["recall:v1", "reflect:v1"])
```

### Behavioral Guidelines

- **Consistency**: Respond in a manner consistent with defined personality across all interactions.
- **Self-reference**: Refer to itself by `name`, never by underlying model or framework name.
- **Boundaries**: Identity definition should include what the agent will NOT do.

### Bootstrap Flow

When a new agent starts for the first time:

1. Check if `identity.yaml` exists. If not, generate one from environment config or prompt the operator.
2. Register itself in the peer registry (see §2.2).
3. Announce presence via the handshake protocol (see §2.2).
4. Initialize the knowledge database (create SQLite tables if missing).

---

## 2. Social Awareness System

In a multi-agent environment, each agent MUST be aware of its social context.

### Peer Registry

```yaml
peers:
  - name: "Agent B"
    uid: "9876543210"
    role: "Research assistant"
    mention_syntax: "<@9876543210>"   # Platform-specific mention format
    status: "active"                   # active | inactive | muted
    capabilities: ["recall:v1"]        # What this peer supports
    notes: "Specializes in summarization"
```

### Peer Discovery & Handshake Protocol

Static `peers.yaml` maintenance does not scale. Agents SHOULD support dynamic discovery:

1. **Announce**: When an agent comes online, it broadcasts a handshake message to the channel (e.g. a structured message or reaction) containing its `name`, `uid`, `role`, and `capabilities`.
2. **Listen**: All agents listen for handshake messages and update their local peer registry accordingly.
3. **Heartbeat** (optional): Agents MAY periodically re-announce to signal liveness. Peers not seen within a configurable TTL are marked `inactive`.

The handshake format is platform-specific. For Discord, this could be a message with a specific prefix or embed. The key requirement is that agents can parse it programmatically.

### Social Rules

- **Discovery**: Agents query the peer registry to find who can help with a given task.
- **Mention Protocol**: Use the platform's `mention_syntax` when referencing another agent.
- **Delegation**: An agent MAY delegate tasks to peers with user consent.
- **Mute/Ignore**: Agents MUST respect mute directives.

---

## 3. Knowledge System

The knowledge system is the agent's long-term memory, designed around three layers:

```
┌─────────────────────────────────────┐
│           SQLite Index              │  ← Fast lookup, search, metadata
│  (paths, tags, timestamps, links)   │
├─────────────────────────────────────┤
│         Knowledge Files (.md)       │  ← Refined, structured knowledge
│  (topics, facts, how-tos, people)   │
├─────────────────────────────────────┤
│      Daily Logs (YYYY-MM-DD.md)     │  ← Raw observations, conversations
│  (unprocessed, timestamped entries) │
└─────────────────────────────────────┘
```

### Scope: Per-Agent vs Shared

Each agent maintains its **own** knowledge base by default (per-agent). Shared knowledge is an optional extension:

- **Per-agent** (default): Each agent has its own `knowledge/`, `logs/`, and `memory.db`. No concurrency issues.
- **Shared** (optional): Multiple agents read/write a common knowledge base. Requires conflict resolution (see §6.3).

Implementors MUST document which mode they use.

### Layer 1: Daily Logs (Raw Input)

**Format**: `logs/YYYY-MM-DD.md`

```markdown
# 2026-04-23

## 14:32 - Conversation with pahud
- Discussed agent cognitive architecture
- Key idea: knowledge should be layered (raw → refined → indexed)
```

Rules:
- Append-only during the day. Never edit past entries.
- Each entry has a timestamp and brief context.

**Log Rotation**: If a single day's log exceeds a configurable threshold (e.g. 500 entries or 100KB), split into `YYYY-MM-DD-001.md`, `YYYY-MM-DD-002.md`, etc. The `daily_logs` table tracks all parts.

### Layer 2: Knowledge Files (Refined)

**Format**: `knowledge/<topic>.md`

```markdown
# SQLite for Agent Memory

## Summary
SQLite with FTS5 is effective for indexing markdown-based knowledge files.

## Key Points
- Use FTS5 for full-text search across knowledge base
- Store file paths, tags, and last-updated timestamps

## Changelog
- 2026-04-23: Created from daily log observation
```

Rules:
- Each file covers ONE topic or entity.
- Files are living documents — refined over time.
- Include a `Changelog` section to track evolution.

### Layer 3: SQLite Index (Fast Lookup)

```sql
CREATE TABLE knowledge_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    tags TEXT,
    summary TEXT,
    content TEXT,                        -- full text content from .md file
    visibility TEXT DEFAULT 'private',   -- private | shared | public
    created_at TEXT NOT NULL,            -- ISO 8601
    updated_at TEXT NOT NULL,
    last_reflected_from TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    title, tags, summary, content,
    content='knowledge_files',
    content_rowid='id'
);

CREATE TABLE daily_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,                  -- YYYY-MM-DD
    part INTEGER DEFAULT 1,             -- for log rotation
    path TEXT NOT NULL UNIQUE,
    status TEXT DEFAULT 'pending',       -- pending | processing | done
    checkpoint TEXT,                     -- last processed timestamp within the log
    updated_at TEXT NOT NULL
);
```

### Index Synchronization

Since `.md` files are the source of truth, the SQLite index can drift if files are modified externally. Implementors SHOULD adopt one of:

1. **File watcher**: Monitor `knowledge/` and `logs/` for changes, trigger re-index on modification.
2. **Periodic rebuild**: Run a scheduled re-index (e.g. on agent startup or via cron).
3. **Hash check**: Store file content hashes in `knowledge_files`; compare on read and re-index if mismatched.

---

## 4. Knowledge Tools

Three commands power the knowledge lifecycle:

### `/recall` — Retrieve Knowledge

1. Parse query into search terms.
2. Query `knowledge_fts` for matching files.
3. Read top-N matching `.md` files.
4. Synthesize and return relevant information.

Search priority: FTS5 keyword search is the default. Implementors MAY add vector/embedding-based semantic search as a fallback or enhancement (see §6.4).

### `/remember` — Store New Knowledge

1. Append raw info to today's daily log.
2. Determine if this fits an existing knowledge file or needs a new one.
3. Update or create the knowledge `.md` file.
4. Update the SQLite index.

### `/reflect` — Extract & Refine Knowledge

1. Find all daily logs where `status = 'pending'`.
2. For each unprocessed log, set `status = 'processing'`.
3. Read raw entries from the checkpoint (or beginning if no checkpoint).
4. Identify discrete knowledge points (facts, preferences, decisions, learnings).
5. For each knowledge point:
   - If a related knowledge file exists → update it.
   - If no related file exists → create a new one.
6. Update the SQLite index for all affected files.
7. Update `checkpoint` after each successfully processed entry.
8. Set `status = 'done'` when the entire log is processed.
9. Return a summary of changes.

**Trigger modes**:
- **Manual**: User invokes `/reflect` explicitly.
- **Scheduled**: Cron or timer-based (e.g. daily at midnight).
- **Threshold**: Auto-trigger when unprocessed log entries exceed a configurable count.

If `/reflect` crashes mid-execution, the `checkpoint` field allows resumption from the last successful entry, avoiding duplicate processing and wasted tokens.

---

## 5. Lifecycle

```
User interaction / Events
        │
        ▼
  ┌──────────┐    /remember
  │ Daily Log │ ◄──────────── Immediate capture
  │ (raw)     │
  └────┬─────┘
       │
       │  /reflect (manual, scheduled, or threshold)
       ▼
  ┌──────────────┐
  │ Knowledge    │ ◄── Extract, merge, refine
  │ Files (.md)  │
  └──────┬───────┘
         │
         │  Index on change
         ▼
  ┌──────────────┐
  │ SQLite Index │ ◄── Fast search & retrieval
  │ (FTS5)       │
  └──────┬───────┘
         │
         │  /recall
         ▼
    Agent Response
```

---

## 6. Implementation Notes

### 6.1 File-First Principle

Knowledge lives in `.md` files. SQLite is an index, not the source of truth. If the DB is lost, rebuild from files.

### 6.2 Idempotent Reflect

Running `/reflect` multiple times on the same log produces the same result. Use the `status` field, `checkpoint`, and changelog entries to prevent duplication.

### 6.3 Conflict Resolution

For **per-agent** knowledge bases (default), no conflict resolution is needed.

For **shared** knowledge bases, implementors MUST choose a strategy:

| Strategy | Pros | Cons | When to use |
|----------|------|------|-------------|
| **Last-write-wins** | Simple | Data loss risk | Low-contention environments |
| **File-level locking** | Prevents concurrent writes | Blocking, deadlock risk | Small teams, low throughput |
| **Merge with changelog** | Auditable, preserves history | Complex implementation | Medium contention |
| **CRDT-inspired** | Conflict-free by design | High complexity | High contention, distributed |

At minimum, all writes MUST append to the `Changelog` section for auditability.

### 6.4 Search: FTS5 vs Embeddings

| Layer | Type | Default | Use case |
|-------|------|---------|----------|
| **FTS5** | Keyword search | ✅ Required | Exact matches, tag lookups, fast filtering |
| **Embeddings** | Semantic search | Optional | Fuzzy/conceptual queries, "find similar" |

When both are available, the recommended flow is: query embeddings first for candidate ranking, then use FTS5 for precision filtering.

```sql
-- Optional: embedding table
CREATE TABLE knowledge_embeddings (
    file_id INTEGER REFERENCES knowledge_files(id),
    chunk_index INTEGER,
    embedding BLOB,
    chunk_text TEXT
);
```

### 6.5 Platform-Agnostic

No assumption on LLM, framework, or platform. This spec works with any agent runtime.

### 6.6 Privacy & Visibility

Knowledge files have a `visibility` field:

- **`private`** (default): Only the owning agent can read/write.
- **`shared`**: All agents in the same workspace can read; only the owner can write.
- **`public`**: All agents can read and write.

Implementors SHOULD enforce visibility at the query layer. Sensitive knowledge SHOULD be encrypted at rest.

---

## 7. File Structure Reference

```
~/
├── identity.yaml              # Self-identity definition (§1)
├── peers.yaml                 # Social peer registry (§2)
├── knowledge/                 # Refined knowledge files (§3)
│   ├── sqlite-memory.md
│   └── agent-architecture.md
├── logs/                      # Daily raw logs (§3)
│   ├── 2026-04-22.md
│   ├── 2026-04-23-001.md      # Log rotation example
│   └── 2026-04-23-002.md
└── memory.db                  # SQLite index (§3)
```

---

## Alternatives Considered

### 1. Pure Database Approach (SQLite/Postgres as source of truth)

Rejected because:
- Less human-readable and harder to debug
- Vendor lock-in to specific DB tooling
- Harder to version control (git-friendly `.md` files are preferred)

### 2. Vector-Only Memory (Embeddings without FTS5)

Rejected as the sole approach because:
- Requires an embedding model dependency (not all agents have access)
- Keyword/exact-match queries are faster and more predictable for structured lookups
- Retained as an optional enhancement layer (§6.4)

### 3. Centralized Knowledge Service (API-based shared memory)

Rejected as the default because:
- Adds infrastructure complexity and a single point of failure
- Not all deployments have a shared backend
- Per-agent file-based storage is simpler and works everywhere
- Retained as an optional shared mode (§3 Scope)

### 4. No Formal Spec (Let each agent figure it out)

Rejected because:
- The whole point of OpenAB is multi-agent interoperability
- Without a shared standard, agents cannot discover peers, share knowledge, or maintain consistent identities

---

## Consequences

- All OpenAB-compatible agents gain a standard way to define identity, discover peers, and manage knowledge.
- Agents can be swapped or upgraded without losing accumulated knowledge (file-first design).
- New agents can bootstrap quickly via the identity + handshake flow.
- The spec is intentionally minimal — implementors can extend it (e.g. vector embeddings, shared knowledge, CRDT) without breaking compatibility.
- Daily log + reflect pattern enables both real-time capture and batch knowledge refinement.
- The `spec_version` field enables future evolution with backward compatibility checks.
