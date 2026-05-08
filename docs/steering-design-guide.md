# Steering Design Guide

How to decide what goes into hot memory (always loaded) vs cold storage (searched on demand) for AI coding agents.

Applies to: Kiro, Claude Code, Codex, Gemini, Copilot, OpenCode — any agent that supports persistent instruction files.

---

## Terminology

| Term | Meaning | Examples |
|------|---------|---------|
| **Hot memory** | Loaded every session, always in context | `AGENTS.md`, `.kiro/steering/`, `CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md` |
| **Cold storage** | Indexed/searchable, loaded on demand | Knowledge bases, `docs/`, project wikis |

---

## What Goes in Hot Memory

| Criteria | Example |
|----------|---------|
| Every interaction might trigger it | Output format spec, verdict logic |
| Identity & relationships | Agent name, team members, contact IDs |
| SOP trigger words | "review PRs" → auto-execute workflow |
| Hard rules that are easy to get wrong | "NITs are blocking", "never merge", "English only on GitHub" |
| Tool usage patterns | Login flows, API call patterns |
| Constraints that override defaults | "Don't ask for confirmation on X", "Always do Y before Z" |

## What Stays in Cold Storage

| Criteria | Example |
|----------|---------|
| Historical records / case studies | Past incident lessons, collaboration logs |
| One-time reference | Installation steps, migration guides |
| Large data | User profiles, conversation history |
| Design proposals / RFCs | Architecture decisions, feature specs |
| Lookup tables | Feature flags, config reference, changelogs |

---

## Design Principles

1. **Small and precise** — Keep hot memory concise. Practical caps vary by agent (CC: ~200 lines for MEMORY.md, Codex: 32KB, Kiro: ~15KB recommended). Regardless of hard limits, attention dilution is the real constraint — less is more.
2. **Behavior-oriented** — Every line should change "what the agent does next." Remove anything that's just "nice to know."
3. **Single source of truth** — Define each rule in exactly one place. Duplication across files creates contradiction risk.
4. **Testable** — Each rule should be verifiable with a single prompt from a fresh session.
5. **One file per responsibility** — Separate concerns: identity, review process, workflow triggers. Avoid monolithic instruction files.
6. **Hot/cold separation** — If the agent can find it via search when needed, it doesn't need to be always-loaded.

---

## Decision Flowchart

```
"If this rule is NOT loaded, will the next response be wrong?"
│
├─ Yes → Hot memory
│
├─ Sometimes, depends on task → Hot memory (if small) or cold with reliable trigger
│
└─ No, it's reference → Cold storage
```

---

## Architecture Pattern

```
Hot (always loaded)              Cold (search on demand)
───────────────────────          ──────────────────────────
AGENTS.md / CLAUDE.md / GEMINI.md  Knowledge bases (semantic search)
.kiro/steering/*.md              docs/*.md
.github/copilot-instructions.md  Project wikis, ADRs, RFCs
MEMORY.md index (CC/Gemini)      Individual memory files
```

> **Real-world example:** Claude Code's auto-memory system is a natural implementation of hot/cold separation — `MEMORY.md` index (hot, 200-line cap) points to individual `.md` memory files (cold, loaded on demand). This pattern validates the guide's core principle.

> **Common pattern:** CC, Codex, and Gemini all use hierarchical loading (global → project → subdir). This naturally supports "one file per responsibility" by placing topic-specific rules in the relevant subdirectory's instruction file.

---

## Agent-Specific File Mapping

> **Note:** Most agents are hybrid — they combine multiple loading models. The table below shows the primary mechanisms.

### Loading Models

| Model | Trigger | Examples |
|-------|---------|---------|
| **Always loaded** | Every session/interaction in repo context | Kiro `.kiro/steering/*`, CC/Codex/Gemini root instruction file, Copilot `.github/copilot-instructions.md` |
| **Directory-scoped** | Processing files within that directory tree | CC/Codex/Gemini subdir instruction files, Copilot `AGENTS.md` (nearest-in-tree) |
| **File-scoped** | Matching an `applyTo` glob pattern | Copilot `.github/instructions/**/*.instructions.md` |

**Implication for hot memory design:**
- "Always loaded" = put task-agnostic rules here (identity, verdict logic, workflow triggers)
- "Directory-scoped" = put domain-specific rules here (gateway checklist, docs standards)
- "File-scoped" = put file-type-specific review expectations here (only Copilot supports this natively)

| Agent | Hot Memory Location | Notes |
|-------|-------------------|-------|
| Kiro | `AGENTS.md` + `.kiro/steering/*.md` | Multiple files, one per topic |
| Claude Code | `CLAUDE.md` (project) + `~/.claude/CLAUDE.md` (global) + `MEMORY.md` index | Hierarchical loading (global → project → subdir). Auto-memory index is hot (200-line cap); individual memory files are cold. `settings.json` is config, not instructions |
| Codex | `AGENTS.md` hierarchical (global → project root → subdir) | Each directory loads at most one file. 32KB cap (`project_doc_max_bytes`). Use nested `AGENTS.md` for per-directory responsibility split. No multi-file topic split within same dir |
| Gemini | `GEMINI.md` hierarchical (`~/.gemini/GEMINI.md` global → `./GEMINI.md` project → subdir) + `MEMORY.md` index | Same hierarchical pattern as CC/Codex. Private project memory index is hot; individual memory files are cold |
| Copilot | `.github/copilot-instructions.md` (repo-wide) + `.github/instructions/**/*.instructions.md` (path-specific) + `AGENTS.md` (nearest-in-tree, where supported: cloud agent / CLI) | Layered: Personal > Path-specific > Repo-wide > Agent > Organization. No documented hard size cap for Chat/Agent (code review reads first 4K chars only). Keep short (~2 pages recommended) |
| OpenCode | `AGENTS.md` or equivalent | Follows repo convention |

---

## Validation Checklist

After adding or changing hot memory:

1. **Start a fresh session** (no prior context)
2. **Ask a question that triggers the rule** — e.g., "what format should a review comment use?"
3. **Verify the response follows the rule exactly**
4. **Test edge cases** — e.g., "what if there's only one 🟡 finding?"
5. **Check for contradictions** — does the new rule conflict with anything else in hot memory?

If the agent doesn't follow the rule → it's either not loaded, too buried in other content, or ambiguously worded.

---

## Anti-Patterns

| Anti-Pattern | Why It's Bad | Fix |
|--------------|-------------|-----|
| Dumping everything into one file | Critical rules get lost in noise | Split by responsibility |
| Duplicating rules across files | Inevitable contradictions when one is updated | Single source + pointer |
| Putting case studies in hot memory | Wastes context budget on history | Move to docs, reference by lesson only |
| Vague rules ("be helpful") | Untestable, no behavioral change | Make specific and testable |
| Hot memory > 20KB | Diminishing returns, attention dilution | Audit and move cold items out |
| Task-scoped rules in file/directory-scoped locations | Review SOP, response format, collaboration protocol only load when certain files are touched — missing when needed most | Put task-agnostic workflow rules in always-loaded layer, not path-specific |
| Stale links in hot memory | Index points to missing files; fresh session gets dead references | Audit links quarterly; remove or create the target |

---

## Maintenance

- **Quarterly audit**: Review hot memory files. Remove rules that are no longer relevant or have become default behavior.
- **After contradictions**: When agent behavior contradicts a rule, check if it's a loading issue or a conflict with another rule.
- **After new capabilities**: When adding new workflows, decide hot vs cold before writing the doc.
- **Adding a new agent**: Document its loading model and precedence before adding file mappings. Don't assume it works like existing agents.
