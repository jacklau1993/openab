# Slash Commands

OpenAB registers Discord slash commands for session control. These work in both guild threads and DMs.

## Commands

| Command | Description | Requires active session? |
|---------|-------------|--------------------------|
| `/models` | Select the AI model via dropdown menu | Yes |
| `/agents` | Select the agent mode via dropdown menu | Yes |
| `/cancel` | Cancel the current in-flight operation | Yes |
| `/reset` | Reset the conversation session (clear history, start fresh) | Yes |
| `/remind` | Set a one-shot delayed reminder to mention users/roles | No |

All responses are **ephemeral** — only the user who invoked the command sees the reply.

## Platform Support

| Platform | Supported | Notes |
|----------|-----------|-------|
| Discord (guild threads) | ✅ | Commands registered per-guild for instant availability |
| Discord (DMs) | ✅ | Commands registered globally; may take up to 1 hour to appear after first deploy |
| Slack | ❌ | Slack blocks third-party slash commands in threads; see [slack-bot-howto.md](slack-bot-howto.md#slash-commands-are-not-supported-on-slack) |

## How They Work

### `/models` and `/agents`

These read `configOptions` from the ACP `initialize` / `session/new` response and present them as a Discord Select Menu.

When the user picks an option, OpenAB sends `session/set_config_option` to the ACP backend.

**Agent support varies:**

| Agent | `/models` | `/agents` |
|-------|-----------|-----------|
| kiro-cli | ✅ Returns available models via `models` fallback | ✅ Returns modes (`kiro_default`, `kiro_planner`) via `modes` fallback |
| claude-code | ❌ No `configOptions` emitted | ❌ |
| codex | ❌ | ❌ |
| gemini | ❌ | ❌ |
| cursor-agent | ❌ (tracking: #493) | ❌ |
| copilot | ❌ (tracking: #496) | ❌ |

If the agent doesn't expose options, the user sees: `⚠️ No model options available. Start a conversation first by @mentioning the bot.`

> **Note:** Discord Select Menus are limited to 25 items. If the agent returns more, only the first 25 are shown with a count of how many were truncated.

### `/cancel`

Sends a `session/cancel` JSON-RPC notification to the ACP backend. This aborts in-flight LLM requests and tool calls immediately — no need to wait for the current response to finish.

### `/reset`

Cancels any in-flight operation, then removes the session from the pool. The ACP process terminates once the last reference is released. The next message in the thread or DM will automatically create a fresh session.

This is equivalent to the `sessions close` + `sessions new` pattern used by [OpenClaw ACPX](https://github.com/openclaw/acpx).

**What gets cleared:**
- Conversation history
- ACP process and connection
- Suspended session state (no resume after reset)

**What is preserved:**
- Bot identity and system prompt (re-applied on next session creation)
- Config settings in `config.toml`

## Passing CLI Commands via @mention

In addition to slash commands, you can pass built-in CLI commands directly after an @mention:

```
@MyBot /compact
@MyBot /clear
@MyBot /model claude-sonnet-4
```

These are forwarded as-is to the ACP session as a prompt. Any command the underlying CLI supports in its interactive mode works here. This is the recommended workaround for agents that don't expose `configOptions`.

## `/remind`

Set a one-shot delayed reminder that mentions users or roles in the channel after a specified delay.

**Syntax:**
```
/remind targets:<@user @role ...> message:<text> delay:<duration>
```

**Parameters:**

| Parameter | Required | Description |
|-----------|----------|-------------|
| `targets` | Yes | Space-separated @mentions (users and/or roles) |
| `message` | Yes | Reminder text |
| `delay` | Yes | Duration before firing: `1m` to `30d` (supports `m`, `h`, `d` and combinations like `1h30m`) |

**Constraints:**
- Only humans can use `/remind` (bots are rejected)
- Minimum delay: 1 minute
- Maximum delay: 30 days
- One-shot only (fires once, then removed)
- Reminders persist across bot restarts (stored in `$HOME/.openab/reminders.json`)

**Examples:**
```
/remind targets:@超渡法師 @普渡法師 message:Review PR #42 delay:2h
/remind targets:@法師們 message:Stand-up time delay:30m
/remind targets:@pahud message:Check deployment delay:1d
```

**When fired, the bot posts:**
```
⏰ Reminder from @sender:
"Review PR #42"
cc @超渡法師 @普渡法師
```
