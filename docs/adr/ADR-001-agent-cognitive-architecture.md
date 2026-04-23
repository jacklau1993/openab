# ADR-001: Agent Cognitive Architecture Specification (ACAS)

- **Status**: Proposed
- **Spec Version**: 1.1.0
- **Date**: 2026-04-23
- **Author**: pahud.hsieh
- **Revision**: Incorporates review feedback from 周嘟嘟, 小喬, 諸葛亮, shaun-agent screening

## Key Words

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## Conformance Levels

This specification defines three conformance levels. Implementors MUST declare which level they target.

| Level | Name | Requirements |
|-------|------|-------------|
| **Level 1** | Identity + Recall | Self-Identity (§1), `/recall` (§4.1), Knowledge Files + SQLite Index (§3) |
| **Level 2** | Full Knowledge | Level 1 + `/remember` (§4.2), `/reflect` (§4.3), Peer Registry (§2) |
| **Level 3** | Shared Knowledge | Level 2 + Shared knowledge mode (§3.1), Peer Discovery Handshake (§2.2) |

---

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

### Required vs Optional Identity Fields

```yaml
spec_version: "1.1.0"           # REQUIRED — spec version this identity conforms to
identity:
  name: ""                       # REQUIRED — agent's name (how it refers to itself)
  uid: ""                        # REQUIRED — unique identifier (platform-specific, e.g. Discord UID)
  persona: ""                    # REQUIRED — one-line self-description
  personality: []                # RECOMMENDED — list of personality traits
  tone: ""                       # OPTIONAL — communication style (e.g. humorous, formal, blunt)
  language: []                   # RECOMMENDED — preferred languages, in order
  origin: ""                     # OPTIONAL — backstory or origin
  capabilities: []               # REQUIRED — supported capabilities (see §1.2)
  boundaries: []                 # RECOMMENDED — what the agent will NOT do
```

### 1.1 Behavioral Guidelines

- **Consistency**: Agents MUST respond in a manner consistent with defined personality across all interactions.
- **Self-reference**: Agents MUST refer to themselves by `name`, never by underlying model or framework name.
- **Boundaries**: Identity definition SHOULD include what the agent will NOT do.

### 1.2 Capability Version Format

Capabilities MUST use the format `<tool>:v<major>` where `<tool>` is the tool name and `<major>` is an integer major version number.

Examples: `["recall:v1", "remember:v1", "reflect:v1"]`

Matching rules:
- **Exact match**: `recall:v1` matches only `recall:v1`.
- **Forward compatibility**: An agent advertising `recall:v2` MAY accept `recall:v1` requests, but this is not guaranteed.
- Freeform strings (e.g. `recall:1.0`, `recall:latest`) are NOT valid.

### 1.3 Bootstrap Flow

When a new agent starts for the first time:

1. Check if `identity.yaml` exists. If not, generate one from environment config or prompt the operator.
2. Register itself in the peer registry (see §2).
3. If Level 3: announce presence via the handshake protocol (see §2.2).
4. Initialize the knowledge database (create SQLite tables if missing).

### 1.4 Migration Note

Agents that currently store identity information in `config.toml` or system prompts MAY continue to do so. The `identity.yaml` file is the canonical source for ACAS-conformant identity. Implementors SHOULD provide a migration path or adapter that reads existing config and produces a conformant `identity.yaml`.

---

## 2. Social Awareness System

In a multi-agent environment, each agent at Level 2+ MUST maintain a peer registry.

### 2.1 Peer Registry

```yaml
peers:
  - name: "Agent B"
    uid: "9876543210"
    role: "Research assistant"
    mention_syntax: "<@9876543210>"   # Platform-specific mention format
    status: "active"                   # active | inactive | muted
    capabilities: ["recall:v1"]        # Capabilities using §1.2 format
    notes: "Specializes in summarization"
```

### 2.2 Peer Discovery & Handshake Protocol

> **Conformance**: This section is REQUIRED for Level 3 only. Level 1–2 agents MAY use a static `peers.yaml` instead.

#### Compatibility with OpenAB Bot Message Filtering

OpenAB's `allow_bot_messages` defaults to ignoring bot messages. Peer discovery MUST NOT assume that bot-to-bot broadcast messages will be received. Instead, agents MUST use one of the following discovery mechanisms:

1. **Shared registry file** (RECOMMENDED): All agents read/write a shared `peers.yaml` or equivalent file on a shared filesystem or object store. On startup, an agent writes its own entry and reads others. Implementations MUST acquire a file-level lock (e.g. `peers.yaml.lock`) before writing registry entries. If the lock is held, the agent MUST wait or skip with a warning — the same semantics as `/reflect` locking (§4.3). Implementations SHOULD ignore invalid or partial updates and retain the last known valid registry state.
2. **Platform API query**: Query the platform API (e.g. Discord guild members) to discover other agents, then populate the local registry.
3. **Operator-managed static config**: The operator maintains `peers.yaml` manually. Simplest approach, no bot-to-bot messaging required.
4. **Mention-triggered exchange** (OPTIONAL): When an agent is @mentioned by another agent, it MAY respond with a structured capability announcement. This works under `allow_bot_messages="mentions"`.

The mention-triggered exchange format for Discord:

```json
{
  "acas_handshake": "v1",
  "name": "Agent B",
  "uid": "9876543210",
  "role": "Research assistant",
  "capabilities": ["recall:v1", "reflect:v1"],
  "status": "active"
}
```

This message MUST be embedded in a Discord message embed or code block. Plain-text reactions MUST NOT be used to carry structured data (Discord reactions only support emoji, not arbitrary payloads).

#### Heartbeat (OPTIONAL)

Agents MAY periodically re-announce to signal liveness. Peers not seen within a configurable TTL MAY be marked `inactive`.

### 2.3 Social Rules

- **Discovery**: Agents query the peer registry to find who can help with a given task.
- **Mention Protocol**: Agents MUST use the platform's `mention_syntax` when referencing another agent.
- **Delegation**: An agent MAY delegate tasks to peers with user consent.
- **Mute/Ignore**: Agents MUST respect mute directives.

---

## 3. Knowledge System

The knowledge system is the agent's long-term memory, designed around three layers:

```
┌─────────────────────────────────────┐
│         SQLite Index                │ ← Fast lookup, search, metadata
│  (paths, tags, timestamps, links)   │
├─────────────────────────────────────┤
│       Knowledge Files (.md)         │ ← Refined, structured knowledge
│  (topics, facts, how-tos, people)   │
├─────────────────────────────────────┤
│     Daily Logs (YYYY-MM-DD.md)      │ ← Raw observations, conversations
│  (unprocessed, timestamped entries) │
└─────────────────────────────────────┘
```

### 3.1 Scope: Per-Agent vs Shared

Each agent maintains its **own** knowledge base by default (per-agent). Shared knowledge is an OPTIONAL extension (Level 3).

- **Per-agent** (default): Each agent has its own `knowledge/`, `logs/`, and `memory.db`. No concurrency issues.
- **Shared** (Level 3): Multiple agents read/write a common knowledge base. REQUIRES conflict resolution (see §6.3).

Implementors MUST document which mode they use.

### 3.2 Layer 1: Daily Logs (Raw Input)

**Format**: `logs/YYYY-MM-DD.md`

```markdown
# 2026-04-23

## 14:32
- Conversation with pahud
- Discussed agent cognitive architecture
- Key idea: knowledge should be layered (raw → refined → indexed)
```

Rules:
- Append-only during the day. Agents MUST NOT edit past entries.
- Each entry MUST have a timestamp and brief context.

**Log Rotation**: If a single day's log exceeds a configurable threshold (default: 100KB), split into `YYYY-MM-DD-001.md`, `YYYY-MM-DD-002.md`, etc. The `daily_logs` table tracks all parts.

### 3.3 Layer 2: Knowledge Files (Refined)

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
- Each file MUST cover ONE topic or entity.
- Files are living documents — refined over time.
- Each file MUST include a `Changelog` section to track evolution.

### 3.4 Layer 3: SQLite Index (Fast Lookup)

```sql
CREATE TABLE knowledge_files (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    path          TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    tags          TEXT,
    summary       TEXT,
    content       TEXT,                              -- full text from .md file
    owner_uid     TEXT NOT NULL,                      -- agent UID that owns this file
    visibility    TEXT NOT NULL DEFAULT 'private',    -- private | shared | public
    created_at    TEXT NOT NULL,                      -- ISO 8601
    updated_at    TEXT NOT NULL,
    last_reflected_from TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    title, tags, summary, content,
    content='knowledge_files',
    content_rowid='id'
);

CREATE TABLE daily_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    date        TEXT NOT NULL,           -- YYYY-MM-DD
    part        INTEGER DEFAULT 1,       -- for log rotation
    path        TEXT NOT NULL UNIQUE,
    status      TEXT DEFAULT 'pending',  -- pending | processing | done
    checkpoint  TEXT,                    -- last processed timestamp within the log
    updated_at  TEXT NOT NULL
);
```

### 3.5 Index Synchronization

Implementors MUST implement at least one of the following strategies to keep the SQLite index in sync with `.md` source files:

1. **File watcher**: Monitor `knowledge/` and `logs/` for changes, trigger re-index on modification.
2. **Startup rebuild**: Re-index all files on agent startup.
3. **Hash check**: Store file content hashes in `knowledge_files`; compare on read and re-index if mismatched.

If the SQLite database is lost or corrupted, the agent MUST be able to rebuild it entirely from the `.md` files. Agents SHOULD verify index integrity on startup and log a warning if drift is detected.

---

## 4. Knowledge Tools

Three commands power the knowledge lifecycle.

### 4.1 `/recall` — Retrieve Knowledge (Level 1+)

1. Parse query into search terms.
2. Query `knowledge_fts` for matching files.
3. Read top-N matching `.md` files.
4. Synthesize and return relevant information.

**Search priority**: FTS5 keyword search is the REQUIRED default. Implementors MAY add embedding-based semantic search as an optional enhancement (see §6.4). When both are available, the recommended flow is: FTS5 first for candidate filtering, then embeddings for re-ranking.

**Visibility enforcement**: Agents MUST filter results by `visibility` and `owner_uid`. An agent MUST NOT return `private` files owned by another agent.

### 4.2 `/remember` — Store New Knowledge (Level 2+)

1. Append raw info to today's daily log.
2. Determine if this fits an existing knowledge file or needs a new one.
3. Update or create the knowledge `.md` file.
4. Update the SQLite index.

All writes MUST set `owner_uid` to the writing agent's UID.

### 4.3 `/reflect` — Extract & Refine Knowledge (Level 2+)

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

#### Concurrency & Failure Handling

- **Locking**: An agent MUST acquire a file-level lock (e.g. `memory.db.lock`) before running `/reflect`. If the lock is held, the agent MUST wait or skip with a warning. This prevents concurrent `/remember` and `/reflect` from corrupting state.
- **Crash recovery**: If `/reflect` crashes mid-execution, the `checkpoint` field records the last successfully processed entry. On next invocation, the agent MUST resume from the checkpoint, not restart from the beginning.
- **Idempotency**: Running `/reflect` multiple times on the same log MUST produce the same result. Use `status`, `checkpoint`, and changelog entries to prevent duplication.

#### Trigger Modes

- **Manual**: User invokes `/reflect` explicitly.
- **Scheduled** (OPTIONAL): Cron or timer-based (e.g. daily at midnight).
- **Threshold** (OPTIONAL): Auto-trigger when unprocessed log entries exceed a configurable count.

---

## 5. Lifecycle

```
User interaction / Events
        │
        ▼
  ┌──────────┐    /remember
  │ Daily Log │ ◄──────────── Immediate capture
  │  (raw)    │
  └────┬─────┘
       │
       │  /reflect (manual, scheduled, or threshold)
       ▼
  ┌──────────────┐
  │  Knowledge   │ ◄── Extract, merge, refine
  │  Files (.md) │
  └──────┬───────┘
         │
         │  Index on change
         ▼
  ┌──────────────┐
  │ SQLite Index │ ◄── Fast search & retrieval
  │   (FTS5)     │
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

Running `/reflect` multiple times on the same log MUST produce the same result. Use the `status` field, `checkpoint`, and changelog entries to prevent duplication.

### 6.3 Conflict Resolution

For **per-agent** knowledge bases (default), no conflict resolution is needed.

For **shared** knowledge bases (Level 3), implementors MUST at minimum:
- Use **file-level locking** to prevent concurrent writes to the same `.md` file.
- **Append to the Changelog section** on every write for auditability.

Implementors MAY additionally adopt one of these strategies:

| Strategy | Pros | Cons | When to use |
|----------|------|------|-------------|
| **Last-write-wins** | Simple | Data loss risk | Low-contention environments |
| **Merge with changelog** | Auditable, preserves history | Complex implementation | Medium contention |
| **CRDT-inspired** | Conflict-free by design | High complexity | High contention, distributed |

### 6.4 Search: FTS5 vs Embeddings

| Layer | Type | Status | Use case |
|-------|------|--------|----------|
| **FTS5** | Keyword search | REQUIRED | Exact matches, tag lookups, fast filtering |
| **Embeddings** | Semantic search | OPTIONAL | Fuzzy/conceptual queries, "find similar" |

When both are available, the recommended flow is: **FTS5 first** for candidate filtering, then embeddings for re-ranking. FTS5 is the required baseline; embeddings are an enhancement.

```sql
-- OPTIONAL: embedding table (non-normative)
CREATE TABLE knowledge_embeddings (
    file_id     INTEGER REFERENCES knowledge_files(id),
    chunk_index INTEGER,
    embedding   BLOB,
    chunk_text  TEXT
);
```

### 6.5 Platform-Agnostic

No assumption on LLM, framework, or platform. This spec works with any agent runtime.

### 6.6 Privacy & Visibility

Knowledge files have a `visibility` field and an `owner_uid` field:

- **`private`** (default): Only the owning agent (`owner_uid`) can read and write.
- **`shared`**: All agents in the same workspace can read; only the owner can write.
- **`public`**: All agents can read and write.

Implementors MUST enforce visibility at the query layer. An agent querying `/recall` MUST NOT return results from `private` files where `owner_uid` does not match the querying agent. Sensitive knowledge SHOULD be encrypted at rest.

---

## 7. File Structure Reference

```
~/
├── identity.yaml          # Self-identity definition (§1)
├── peers.yaml             # Social peer registry (§2)
├── knowledge/             # Refined knowledge files (§3)
│   ├── sqlite-memory.md
│   └── agent-architecture.md
├── logs/                  # Daily raw logs (§3)
│   ├── 2026-04-22.md
│   ├── 2026-04-23-001.md  # Log rotation example
│   └── 2026-04-23-002.md
└── memory.db              # SQLite index (§3)
```

---

## 8. Acceptance Criteria

An implementation is conformant at a given level if it satisfies all of the following for that level:

### Level 1 — Identity + Recall
- [ ] `identity.yaml` exists with all REQUIRED fields populated
- [ ] `capabilities` field uses `<tool>:v<major>` format
- [ ] `knowledge/` directory contains `.md` files with Changelog sections
- [ ] `memory.db` contains `knowledge_files` and `knowledge_fts` tables with `owner_uid` column
- [ ] `/recall` returns results filtered by `visibility` and `owner_uid`
- [ ] SQLite index can be rebuilt entirely from `.md` files
- [ ] At least one index sync strategy is implemented

### Level 2 — Full Knowledge
- [ ] All Level 1 criteria
- [ ] `peers.yaml` exists with at least the agent's own entry
- [ ] `/remember` appends to daily log and updates knowledge files + index
- [ ] `/remember` sets `owner_uid` on all created/updated records
- [ ] `/reflect` processes `pending` logs with three-state tracking (pending → processing → done)
- [ ] `/reflect` uses checkpoint for crash recovery
- [ ] `/reflect` acquires file-level lock before execution
- [ ] `/reflect` is idempotent on the same log
- [ ] `/reflect` resumes from checkpoint after crash without reprocessing already-reflected entries

### Level 3 — Shared Knowledge
- [ ] All Level 2 criteria
- [ ] Peer discovery uses a mechanism compatible with `allow_bot_messages` defaults (§2.2)
- [ ] Shared knowledge writes use file-level locking + changelog append
- [ ] Visibility enforcement is implemented at the query layer

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
- Retained as an optional shared mode (§3.1 Scope)

### 4. No Formal Spec (Let each agent figure it out)
Rejected because:
- The whole point of OpenAB is multi-agent interoperability
- Without a shared standard, agents cannot discover peers, share knowledge, or maintain consistent identities

### 5. Bot-to-Bot Broadcast for Peer Discovery
Rejected as the default because:
- OpenAB's `allow_bot_messages` defaults to ignoring bot messages
- Broadcast-based handshake would fail silently in default deployments
- Retained as an optional mechanism under `allow_bot_messages="mentions"` (§2.2)

---

## Consequences

- All OpenAB-compatible agents gain a standard way to define identity, discover peers, and manage knowledge.
- Agents can be swapped or upgraded without losing accumulated knowledge (file-first design).
- New agents can bootstrap quickly via the identity + handshake flow.
- The spec is intentionally minimal — implementors can extend it (e.g. vector embeddings, shared knowledge, CRDT) without breaking compatibility.
- Daily log + reflect pattern enables both real-time capture and batch knowledge refinement.
- The `spec_version` field enables future evolution with backward compatibility checks.
- Conformance levels allow incremental adoption without requiring full implementation upfront.

---

## Non-Normative Extensions (Future Work)

The following are explicitly out of scope for this ADR and MAY be addressed in follow-up ADRs:

- **Vector/embedding-based semantic search** — schema provided in §6.4 as a starting point
- **CRDT-based conflict resolution** — for high-contention shared knowledge scenarios
- **Scheduled `/reflect` orchestration** — cron/timer integration, retry policies, run isolation
- **Cross-platform peer discovery** — unified discovery across Discord, Slack, and other platforms
- **Knowledge encryption at rest** — detailed key management and encryption scheme
