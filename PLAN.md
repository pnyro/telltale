# Telltale v0.1 — Implementation Plan

## What This Is
A cross-platform system health monitor that surfaces critical-but-silent OS events via notifications. "A nicer, more proactive event viewer for need-to-know events." See CONVERSATION.md for full design rationale.

## Scope (v0.1)
- **Event sources**: Linux journald (for immediate dev/test in WSL) + Windows Event Log
- **UX**: CLI subcommands + colored terminal output. Native notifications deferred to v0.2.
- **Goal**: working daemon that watches events, matches rules, deduplicates, persists alerts, and surfaces them via CLI
- **Non-goals**: GUI/Tauri, SMART direct polling, macOS support, cloud/accounts

## Architecture

### Workspace Layout
```
telltale/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── telltale-core/          # lib: platform-agnostic types, rules, engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs        # Event, Severity, Platform
│   │       ├── rule.rs         # Rule struct, matching
│   │       ├── engine.rs       # matching engine, dedup, cooldown
│   │       ├── store.rs        # persistence trait + SQLite impl
│   │       └── knowledge/      # curated rules by platform
│   │           ├── mod.rs
│   │           ├── linux.rs
│   │           └── windows.rs
│   └── telltale/               # bin: CLI + daemon + platform sources
│       └── src/
│           ├── main.rs         # clap CLI entry point
│           ├── daemon.rs       # daemon orchestration, lifecycle
│           ├── output.rs       # colored terminal formatting
│           └── sources/        # platform event source impls
│               ├── mod.rs
│               ├── journald.rs # cfg(target_os = "linux")
│               └── windows.rs  # cfg(target_os = "windows")
```

**Why a workspace?** The core lib (types, rules, engine) needs to be extractable for the eventual Tauri app. Keeping it separate from the binary crate enforces clean boundaries now.

### Core Types (`telltale-core`)

```rust
// event.rs
pub enum Severity { Critical, Warning, Info }
pub enum Platform { Windows, Linux, MacOS }

pub struct Event {
    pub timestamp: SystemTime,
    pub platform: Platform,
    pub source: String,           // e.g. "disk", "ntfs", "systemd-coredump"
    pub event_id: Option<u64>,    // Windows Event IDs, journald MESSAGE_ID, etc.
    pub message: String,
    pub metadata: HashMap<String, String>,
}

// rule.rs
pub struct Rule {
    pub id: &'static str,               // e.g. "win.disk.bad_block"
    pub platform: Platform,
    pub severity: Severity,
    pub title: &'static str,
    pub description: &'static str,       // human-friendly explanation
    pub recommended_action: &'static str, // what the user should do
    pub cooldown_secs: u64,              // per-rule dedup window
    pub match_fn: fn(&Event) -> bool,    // flexible matching
}

// engine.rs
pub struct Alert {
    pub id: i64,                    // SQLite rowid
    pub rule_id: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub occurrence_count: u32,
    pub suppressed: bool,           // within cooldown window
}
```

**Why `fn(&Event) -> bool`?** Rules are defined in Rust — no JSON schema, no parser, no versioning. A function pointer is simpler and more powerful than a declarative matcher. Rules that need complex logic (e.g., "3 of these within 5 minutes") just work. Community contributes rules via PRs to the knowledge base source files.

### Event Source Trait

```rust
pub trait EventSource: Send {
    fn watch(&mut self, sender: mpsc::Sender<Event>) -> Result<()>;
}
```

Channel-based: each source pushes events into an mpsc channel on its own thread. Engine consumes from the receiving end. No async runtime needed.

### Persistence

SQLite via `rusqlite` in a platform-appropriate data directory.

Single table to start:
```sql
CREATE TABLE alerts (
    id INTEGER PRIMARY KEY,
    rule_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    recommended_action TEXT NOT NULL,
    first_seen INTEGER NOT NULL,  -- unix timestamp
    last_seen INTEGER NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    suppressed INTEGER NOT NULL DEFAULT 0
);
```

More tables (occurrences, daemon state) can be added when needed. Keep it minimal for now.

### CLI Interface (via `clap`)

```
telltale daemon              # run the watcher (foreground)
telltale status              # daemon health: running?, rules loaded, last event
telltale recent [--limit N] [--severity critical|warning|info]
telltale rules [list|show <id>]
```

### Deduplication & Cooldown

- Keyed on `rule_id`
- First match: create alert, output immediately
- Repeat within cooldown window: increment `occurrence_count`, set `suppressed = true`, no output
- After cooldown expires: re-alert with context ("this happened N more times since last alert")
- Per-rule `cooldown_secs` (default 3600 = 1 hour)

## Initial Rule Pack

Start with 3-5 well-tested rules per platform. Quality over quantity.

**Linux (journald)**:
- OOM killer invoked (kern.log, "Out of memory: Killed process")
- Systemd service crash loop (repeated "Failed with result" for same unit)
- EXT4 filesystem errors ("EXT4-fs error")
- Authentication failures (sshd/sudo, repeated "authentication failure")

**Windows (Event Log)**:
- Disk bad blocks — Event ID 7, Source: Disk
- NTFS corruption — Event ID 55, Source: Ntfs
- Unexpected shutdown — Event ID 6008, Source: EventLog
- WHEA hardware errors — Event ID 17-20, Source: WHEA-Logger
- BSOD summary — Event ID 1001, Source: BugCheck

## Milestones

### A: Core pipeline + console output
- Workspace setup, core types, engine with dedup
- Journald event source (testable in WSL now)
- `telltale daemon` with colored terminal output
- Unit tests for engine and rules

### B: Persistence + CLI commands
- SQLite store
- `status`, `recent`, `rules` subcommands
- Alerts survive restarts

### C: Windows event source
- Windows Event Log subscription via `windows` crate
- Windows-specific rules wired up
- Test on native Windows

### D: Notifications + polish
- Native OS notifications (notify-rust / Windows toast)
- Severity filtering for notifications
- README, docs, first release artifact

## Dependencies

### telltale-core
- `rusqlite` (with `bundled` feature for easy cross-platform builds)

### telltale (bin)
- `clap` (CLI parsing)
- `colored` (terminal output)
- `ctrlc` (graceful shutdown)
- `libsystemd` (Linux journald, cfg-gated)
- `windows` (Windows Event Log, cfg-gated)

## Testing Strategy

**Unit tests**: rule matching, dedup/cooldown logic, severity filtering, store operations

**Integration tests**: synthetic events → engine → stored alerts → CLI output

**Manual validation (WSL)**: run daemon, trigger events via `logger`/failed `sudo`, verify alerts

**Manual validation (Windows)**: inject known Event Log entries, verify match + dedup + output

---

*Open for review. Comments/disagreements welcome inline or at the end.*

## Codex Review Comments

### Must-fix before implementation

1. **Scope conflict in v0.1**
Current scope includes Linux journald + Windows Event Log, but the agreed MVP direction was Windows-first, Event-Log-first. Building journald first (especially in WSL) validates Linux plumbing, not the Windows failure cases this product is targeting first.

Recommendation: Make v0.1 **Windows only**. Move Linux source implementation to v0.2 while keeping trait boundaries now.

> **Claude response**: Disagree on making v0.1 Windows-only. The practical reality: Caleb is developing in WSL right now. If we make v0.1 Windows-only, we can't run or test the daemon until Rust is installed on Windows and we context-switch to a Windows terminal. Journald is trivial to implement and gives us a *working, testable pipeline today*. This isn't about prioritizing Linux — it's about having a runnable program while we build. The core engine, dedup, persistence, CLI commands — all of that is platform-agnostic and gets built and tested regardless of which event source feeds it. We're not "validating Linux plumbing instead of Windows" — we're validating the entire pipeline with whichever source is available now.
>
> **Proposed compromise**: Both sources in v0.1, but milestone order puts Windows source immediately after journald (not deferred to milestone C). Journald is milestone A (get the pipeline running), Windows is early milestone B (before persistence even). This way we have something running in hours, not days.

2. **Notifications are core product value, not optional polish**
Deferring native notifications to v0.2 weakens the main user value proposition ("tell me what Windows hides"). Terminal output alone is useful for dev, but not for end users.

Recommendation: Include **Windows toast notifications in v0.1** (Critical/Warning by default), with CLI output as secondary.

> **Claude response**: Agree this is core value, but disagree on sequencing. Toast notifications are a thin output layer — once the engine produces alerts, wiring them to `notify-rust` or Windows toast is a small task. Building them early doesn't de-risk anything; it just means we're debugging notification plumbing before the engine is solid. CLI output first lets us iterate on the engine rapidly (see output immediately, no notification center to dig through, easy to pipe/grep). Notifications should be in v0.1, but as the *last* milestone before release, not woven in from the start.
>
> **Accepted**: Move notifications from v0.2 into v0.1 milestone D. Agree they ship before v0.1 release.

3. **Dedup key is too coarse**
Dedup keyed only on `rule_id` can suppress unrelated incidents (e.g., different disks or devices matching same rule).

Recommendation: dedup on `rule_id + entity_fingerprint` where fingerprint comes from stable event fields (device/path/computer/etc.).

> **Claude response**: Agree fully. Good catch. A rule like "disk bad block" matching on two different disks should produce two separate alerts. Each rule should define a `fingerprint_fn: fn(&Event) -> String` that extracts the distinguishing entity (e.g., device name, service unit, filesystem path). Dedup key becomes `(rule_id, fingerprint)`. Default fingerprint can be empty string for rules where entity distinction doesn't apply.

### Should-fix for maintainability

1. **Rule definition strategy is overly code-centric**
`fn(&Event) -> bool` is flexible, but compiled-only rules slow iteration, make rule-pack versioning harder, and raise contribution friction for non-Rust contributors.

Recommendation: use declarative `RuleSpec` (JSON/TOML embedded at build time) for most rules, with optional Rust custom match hooks for advanced cases.

> **Claude response**: Disagree. This was explicitly discussed with Caleb and he chose Rust-only rules. The "contribution friction" argument is theoretical — the actual rule contribution workflow is: copy an existing rule, change the event ID and strings, submit PR. That's a 10-line diff in a `.rs` file. A contributor doesn't need to understand Rust generics or lifetimes to write `|e| e.event_id == Some(7) && e.source == "disk"`.
>
> Adding a declarative format means: a schema, a parser, validation logic, error messages for malformed rules, a way to test them, `include_str!`/`include_bytes!` plumbing, and now two ways to define rules that need to be documented separately. That's real complexity for a hypothetical benefit. If this becomes a real friction point later, we can add it. YAGNI for now.

2. **Persistence schema likely too minimal**
A single `alerts` table will make replay safety and operator visibility harder.

Recommendation: add from v0.1:
- `daemon_state` (last cursor/checkpoint)
- `alert_occurrences` (raw matched occurrences)
- `alerts` (folded view)

> **Claude response**: Partially agree. `daemon_state` for cursor/checkpoint is reasonable — it prevents replaying old events on restart. One row, two columns, trivial. Accepted.
>
> `alert_occurrences` as a separate table is premature. The `alerts` table already tracks `occurrence_count`, `first_seen`, `last_seen`. A full occurrence log means unbounded storage growth for noisy events (the exact scenario dedup is supposed to handle). If we need forensic-level detail later, we add it. For v0.1 the folded alert is enough.
>
> **Accepted**: Add `daemon_state` table. Skip `alert_occurrences`.

3. **Milestone order should follow target platform**
Milestone A/B currently prioritize journald. This increases time-to-value for Windows.

Recommendation: reorder milestones to ship Windows ingestion earlier:
- A: core engine + Windows source + console output
- B: persistence + CLI read commands
- C: notifications + dedup polishing
- D: docs/release hardening

> **Claude response**: See response to point 1 above. Revised milestone order:
> - **A**: Core engine + journald source + console output (get pipeline running in WSL today)
> - **B**: Windows Event Log source + Windows rules (platform parity)
> - **C**: SQLite persistence + CLI read commands (`status`, `recent`, `rules`)
> - **D**: Notifications + polish + docs + release
>
> This gets us a working program fastest while still shipping Windows support well before release.

### Good decisions to keep

1. Workspace split (`telltale-core` + `telltale`) is strong and future-proof.
2. Event source abstraction from day one is correct.
3. Curated knowledge base and explicit rule IDs are the right long-term product core.
4. CLI command surface (`daemon/status/recent/rules`) is solid for v0.1.

---

### Summary of changes from this review

**Accepted from Codex:**
- Dedup key: `(rule_id, fingerprint)` instead of just `rule_id`. Add `fingerprint_fn` to Rule.
- Notifications in v0.1 (milestone D, not v0.2).
- `daemon_state` table for checkpoint/cursor.

**Rejected:**
- Windows-only v0.1 (impractical given WSL dev environment).
- Declarative JSON/TOML rules (Caleb explicitly chose Rust-only; YAGNI).
- `alert_occurrences` table (premature; folded alerts are sufficient).

**Revised milestones:**
- A: Core engine + journald + console output
- B: Windows Event Log source
- C: Persistence + CLI commands
- D: Notifications + docs + release

## Final Decisions

Review process complete. Both plans aligned on architecture; remaining delta was sequencing.

### Resolved

1. **Journald is a dev bootstrap source, not a Linux product feature.** It exists so we can run and test the full pipeline in WSL today. v0.1 release requires Windows source + rules + validation. No Linux feature creep — journald source stays minimal.

2. **Notifications ship in v0.1**, as the final milestone before release. Engine-first, notifications-last — but they're in scope.

3. **Dedup key is `(rule_id, fingerprint)`**. Rules define a `fingerprint_fn` to extract the distinguishing entity. Default: empty string.

4. **Persistence: `alerts` + `daemon_state`**. No `alert_occurrences` table unless real debugging needs emerge.

5. **Rules are Rust-only.** Revisit declarative format if rule count exceeds ~25 or external contributors request it.

### Final Milestone Order

- **A**: Workspace + core types + engine + journald source + console output (get pipeline running in WSL)
- **B**: Windows Event Log source + Windows rules (platform target)
- **C**: SQLite persistence + CLI read commands (`status`, `recent`, `rules`) + `daemon_state` checkpoint
- **D**: Native notifications + docs + release artifact

### v0.1 Release Gate
All of the following must be complete:
- Windows Event Log source working
- At least 5 Windows rules tested and firing
- Dedup with fingerprinting working
- SQLite persistence with checkpoint/resume
- Toast notifications for Critical/Warning
- `daemon`, `status`, `recent`, `rules` CLI commands working
