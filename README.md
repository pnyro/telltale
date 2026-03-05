# Telltale

A system health monitor that surfaces critical-but-silent OS events. Windows quietly logs disk failures, filesystem corruption, hardware errors, and more — but rarely tells you about them until it's too late. Telltale watches for these events and tells you what matters, in plain English.

Think of it as a nicer, more proactive Event Viewer — but only for the things you actually need to know.

## Status

Early development. The core engine and Linux journald source are working. Windows Event Log support is next.

## Build

Requires [Rust](https://rustup.rs/) (edition 2024).

```
cargo build
```

## Run

Start the daemon to watch for system events in real time:

```
telltale daemon
```

On Linux, this tails `journalctl` and matches against curated rules. Matched alerts are printed to the terminal with severity, explanation, and recommended action.

## Test

```
cargo test --workspace
```

## Architecture

- **telltale-core** (lib) — platform-agnostic event types, rule definitions, matching engine with dedup/cooldown
- **telltale** (bin) — CLI entry point, platform-specific event sources, terminal output

Rules are defined in Rust as function pointers (`fn(&Event) -> bool`), organized by platform under `crates/telltale-core/src/knowledge/`. Each rule includes a human-readable title, description, recommended action, and cooldown period.

## Roadmap

See [PLAN.md](PLAN.md) for the full implementation plan. In short:

- **Milestone A** — Core engine + journald source + console output *(done)*
- **Milestone B** — Windows Event Log source + Windows rules
- **Milestone C** — SQLite persistence + CLI commands (`status`, `recent`, `rules`)
- **Milestone D** — Native OS notifications + docs + release
