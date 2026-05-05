---
'@wecom/cli': minor
---

feat(init): add `--bootstrap` flag for non-interactive provisioning

Adds a new `wecom-cli init --bootstrap` mode that reads Bot credentials from
`WECOM_CLI_BOOTSTRAP_BOT_ID` and `WECOM_CLI_BOOTSTRAP_BOT_SECRET` environment
variables and runs the standard `set_bot_info` + `fetch_mcp_config`
validation path (with the same rollback-on-failure semantics as interactive
mode).

Use cases:

- AI Agents that need to provision the CLI without a human at the terminal
- Sandbox / Docker images that ship with credentials pre-injected via env
- CI pipelines that test against a real WeCom bot

`--bootstrap` and `--noninteractive` are mutually exclusive. Empty or missing
env vars cause an immediate fail-fast bail without touching disk state.
