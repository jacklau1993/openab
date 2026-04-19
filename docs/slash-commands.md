# Slash Commands

OpenAB registers Discord slash commands per guild when the bot connects. These commands interact with the ACP session in the current thread.

## Available Commands

| Command | Description |
|---|---|
| `/models` | Select the AI model for this session |
| `/agents` | Select the agent mode for this session |
| `/cancel` | Cancel the current in-flight operation |

## /models

Opens an ephemeral dropdown with available AI models. Selecting a model switches the session via ACP `session/set_config_option`.

- The dropdown shows the current model as pre-selected
- Options are read from the ACP session's `configOptions` (cached at session creation)
- If no session exists yet, shows "No model options available"
- Falls back to sending `/model <value>` as a prompt if `set_config_option` is not supported

### Backend support

| Backend | configOptions | set_config_option |
|---|---|---|
| Codex (`codex-acp`) | ✅ Native | ✅ Native |
| Kiro (`kiro-cli`) | ⚠️ `models`/`modes` fallback | ⚠️ Prompt fallback (`/model <value>`) |

## /agents

Opens an ephemeral dropdown with available agent modes (e.g. `kiro_default`, `kiro_planner`).

- Works the same as `/models` but for the `agent` category
- Falls back to `/agent <value>` prompt for backends without native support

## /cancel

Sends a `session/cancel` JSON-RPC notification to the ACP agent, aborting any in-flight LLM requests and tool calls.

- Response is ephemeral (only visible to you)
- Must be used in a thread with an active session
- If no session exists, shows an error message

## Requirements

- The bot must have the **Use Slash Commands** permission in the channel
- A session must be active in the thread (start one by @mentioning the bot)
- Slash commands are registered per guild on bot connect — if commands don't appear, try kicking and re-inviting the bot

## How configOptions work

```
User @mentions bot → session/create → ACP returns configOptions
                                       ↓
                              OpenAB caches options
                                       ↓
User runs /models → OpenAB reads cache → renders dropdown
                                       ↓
User selects      → session/set_config_option (or prompt fallback)
                                       ↓
                              Cache updated from response
```

Options are parsed from the ACP `session/create` response:
- **Standard ACP**: `result.configOptions[]` array
- **Kiro fallback**: `result.models` and `result.modes` objects

During streaming, `config_option_update` notifications keep the cache in sync.
