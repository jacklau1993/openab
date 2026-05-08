# Steering Design Guide

How to decide what goes into hot memory (always loaded) vs cold storage (searched on demand) for AI coding agents.

Applies to: Kiro, Claude Code, Codex, Gemini, Copilot, OpenCode — any agent that supports persistent instruction files.

---

## Terminology

| Term | Meaning | Examples |
|------|---------|---------|
| **Hot memory** | Loaded every session, always in context | `AGENTS.md`, `.kiro/steering/`, `CLAUDE.md`, `.codex/instructions.md`, `.github/copilot-instructions.md` |
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

1. **Small and precise** — Keep hot memory under ~15KB total. Larger context dilutes attention on critical rules.
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
AGENTS.md / CLAUDE.md            Knowledge bases (semantic search)
.kiro/steering/*.md              docs/*.md
.codex/instructions.md           Project wikis
.github/copilot-instructions.md  ADRs, RFCs, lessons learned
```

---

## Agent-Specific File Mapping

| Agent | Hot Memory Location | Notes |
|-------|-------------------|-------|
| Kiro | `AGENTS.md` + `.kiro/steering/*.md` | Multiple files, one per topic |
| Claude Code | `CLAUDE.md` + `.claude/settings.json` | Single file + config |
| Codex | `.codex/instructions.md` | Single file |
| Gemini | `GEMINI.md` or context window | Varies by integration |
| Copilot | `.github/copilot-instructions.md` | Single file |
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

---

## Maintenance

- **Quarterly audit**: Review hot memory files. Remove rules that are no longer relevant or have become default behavior.
- **After contradictions**: When agent behavior contradicts a rule, check if it's a loading issue or a conflict with another rule.
- **After new capabilities**: When adding new workflows, decide hot vs cold before writing the doc.
