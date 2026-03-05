# Telltale

Telltale is a proactive system event monitor. It watches for high-signal OS events that are often logged silently — disk failures, filesystem corruption, hardware errors, unexpected shutdowns — and surfaces them as actionable alerts with plain-English explanations.

Think of it as a nicer, more proactive Event Viewer for the things you actually need to know.

## How it works

Telltale runs as a background daemon that subscribes to your OS event log in real time. When an event matches one of its curated rules, it:

1. Classifies the event by severity (Critical, Warning, Info)
2. Deduplicates repeated occurrences using per-rule cooldown windows
3. Persists the alert to a local SQLite database
4. Fires a native toast notification (Critical and Warning only)
5. Prints a colored summary to the terminal

Alerts are deduplicated by `(rule_id, fingerprint)` — so the same rule firing for two different disks produces two separate alerts, but the same disk failing repeatedly is consolidated.

## Platform support

| Platform | Status | Notes |
|---|---|---|
| Windows | Full | Event Log source, 5 curated rules, toast notifications |
| Linux | Dev/experimental | journald source, 4 rules, no native notifications yet |
| macOS | Not implemented | Planned |

## Getting started

Requires [Rust](https://rustup.rs/) (edition 2024).

```bash
cargo build
```

Start the daemon:

```bash
telltale daemon
```

Or run a simulation to see the full pipeline in action:

```bash
telltale simulate --interval 2 --count 5
```

## CLI commands

| Command | Description |
|---|---|
| `telltale daemon` | Run the event monitor (foreground) |
| `telltale simulate [--interval SECS] [--count N]` | Generate synthetic events through the full pipeline |
| `telltale status` | Show daemon database status, rule count, last checkpoint |
| `telltale recent [--limit N] [--severity LEVEL]` | Display recent alerts from the database |
| `telltale rules list` | List all rules for the current platform |
| `telltale rules show <id>` | Show full detail for a specific rule |

## Architecture

```
telltale/
├── crates/
│   ├── telltale-core/     # Platform-agnostic library
│   │   ├── engine.rs      # Matching engine with dedup and cooldown
│   │   ├── store.rs       # SQLite persistence (alerts + daemon state)
│   │   ├── rule.rs        # Rule type with fn-pointer matching
│   │   └── knowledge/     # Curated rules by platform
│   └── telltale/          # CLI binary
│       ├── daemon.rs      # Daemon orchestration and main loop
│       ├── notify.rs      # Notification trait + Windows toast impl
│       └── sources/       # Event source trait + platform impls
```

The core library is separated from the binary to support future GUI clients (Tauri).

## Data storage

Alerts are persisted to SQLite in the platform data directory:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\telltale\telltale.db` |
| Linux | `~/.local/share/telltale/telltale.db` |
| macOS | `~/Library/Application Support/telltale/telltale.db` |

## Troubleshooting

**Notifications go to Action Center but don't show as banners**
- Check that Do Not Disturb / Focus Assist is off in Windows notification settings.
- Go to Settings → System → Notifications → Telltale and verify "Show notification banners" is enabled.

**No notifications at all**
- Run `telltale simulate --count 1` and check stderr for `notification error:` or `warning: failed to register app` messages.
- Telltale registers itself in the Windows registry on first run (`HKCU\Software\Classes\AppUserModelId\Telltale.SystemMonitor`). If this fails due to permissions, notifications will be silently dropped.

**Daemon shows "no rules available" on startup**
- Rules are platform-specific. Running on an unsupported OS will produce this error.

**All simulated alerts are suppressed**
- Previous alerts within the cooldown window are restored from the database on startup. Delete the database file to reset: `telltale status` shows the database path.

## Contributing

Rule contributions are the most valuable kind of contribution. Each rule maps a specific OS event to a human-readable explanation and recommended action.

1. Pick the platform rules file:
   - `crates/telltale-core/src/knowledge/windows.rs`
   - `crates/telltale-core/src/knowledge/linux.rs`
2. Add a `match_fn` (what to match) and `fingerprint_fn` (what entity to deduplicate on).
3. Add a `Rule` entry with a unique `id`, `severity`, user-facing `title`, `description`, `recommended_action`, and sensible `cooldown_secs`.
4. Add unit tests for the match function.
5. Run `cargo fmt` and `cargo test --workspace`.

See [docs/rules.md](docs/rules.md) for the full rule structure and current rule inventory.

## License

TBD
