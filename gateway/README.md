# OpenAB Custom Gateway

Standalone gateway service that bridges webhook-based platforms (Telegram, LINE, GitHub, etc.) to OAB via WebSocket.

See [ADR: Custom Gateway](../docs/adr/custom-gateway.md) for architecture details.

## Quick Start

```bash
# Build
cargo build --release

# Run (Telegram)
export TELEGRAM_BOT_TOKEN="your-bot-token"
./target/release/openab-gateway
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `TELEGRAM_BOT_TOKEN` | (required) | Telegram Bot API token |
| `GATEWAY_LISTEN` | `0.0.0.0:8080` | Listen address |
| `TELEGRAM_WEBHOOK_PATH` | `/webhook/telegram` | Webhook endpoint path |

## Endpoints

| Path | Description |
|---|---|
| `POST /webhook/telegram` | Telegram webhook receiver |
| `GET /ws` | WebSocket server (OAB connects here) |
| `GET /health` | Health check |

## OAB Config

```toml
[gateway]
url = "ws://gateway:8080/ws"
```

## Setting Up the Telegram Webhook

```bash
curl "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/setWebhook?url=https://your-gateway-host/webhook/telegram"
```

> **Note:** Telegram requires HTTPS. Use a reverse proxy, Cloudflare Tunnel, or similar for TLS termination.
