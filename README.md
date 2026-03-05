# Telltale

Telltale is a proactive system event monitor. It watches for high-signal OS events that are often logged silently and surfaces them with actionable output.

## Notification behavior

- `Critical` and `Warning` alerts trigger notifications by default.
- `Info` alerts are stored and shown in CLI output, but do not notify.
- Duplicate events are deduplicated by `(rule_id, fingerprint)` and cooldown windows.
- Notifications are fire-and-forget and do not block the daemon loop.

## Platform support

| Platform | Status | Notes |
|---|---|---|
| Windows | Full (v0.1 scope) | Event Log source, Windows rules, toast notifications |
| Linux | Dev/experimental | journald source for development bootstrap; no native notifications yet |
| macOS | Not implemented | Planned later |

## CLI usage

Build:

```bash
cargo build
```

Run daemon:

```bash
cargo run -p telltale -- daemon
```

Show status:

```bash
cargo run -p telltale -- status
```

Show recent alerts:

```bash
cargo run -p telltale -- recent --limit 20
cargo run -p telltale -- recent --limit 10 --severity critical
```

List and inspect rules:

```bash
cargo run -p telltale -- rules list
cargo run -p telltale -- rules show win.disk.bad_block
```

Run tests:

```bash
cargo test --workspace
```

## Architecture

- `crates/telltale-core`: core types, matching engine, rules, and SQLite store
- `crates/telltale`: CLI app, event sources, notification adapters, daemon orchestration

## Contributing

Rule contributions are welcome.

1. Pick the platform rules file:
- `crates/telltale-core/src/knowledge/windows.rs`
- `crates/telltale-core/src/knowledge/linux.rs`
2. Add a `match_fn` and `fingerprint_fn`.
3. Add a `Rule` entry with:
- unique `id`
- `severity`
- user-facing `title`, `description`, and `recommended_action`
- sensible `cooldown_secs`
4. Add unit tests for the new rule match behavior.
5. Run `cargo fmt` and `cargo test --workspace`.

See [docs/rules.md](docs/rules.md) for detailed rule structure and current rule inventory.

## Roadmap

See [PLAN.md](PLAN.md) for milestone context and sequencing.
