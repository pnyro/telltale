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

### v0.1 Release Gate — COMPLETE
All of the following are complete:
- Windows Event Log source working
- 5 Windows rules tested and firing
- Dedup with fingerprinting working
- SQLite persistence with checkpoint/resume
- Toast notifications for Critical/Warning
- `daemon`, `status`, `recent`, `rules`, `simulate` CLI commands working

---

## v0.2 — Real-world validation & expanded knowledge base

### Context
v0.1 shipped with 5 Windows rules targeting rare-but-critical hardware events (disk bad blocks, NTFS corruption, WHEA, unexpected shutdown, BugCheck). Testing against a real Windows machine showed only 3 of 5 matching (the hardware ones require actual failures). Meanwhile, the Event Log contains dozens of **Warning/Error-level operational events** that are common, actionable, and invisible to users — the exact gap telltale exists to fill.

v0.2 focuses on: (1) making telltale useful on a real machine today via historical scan, (2) expanding the rule set based on real observed events, and (3) building a capture/replay harness so every rule ships with a real event fixture.

### Goals
- `telltale scan` command for one-shot historical analysis
- Event capture/export for building test fixtures from real machines
- Expand Windows rules from 5 to ~15 based on real Event Log data
- Every rule has a test fixture from a captured real event

### Non-goals (deferred)
- Linux notifications, macOS support
- Tauri GUI (follows once core is settled)
- License selection (repo is private)

### New Windows rules to add (based on real Event Log analysis)

**Tier 1 — High value, commonly seen:**

| Rule ID | Event ID | Source | Severity | What it catches |
|---|---|---|---|---|
| `win.kernel_power.dirty_reboot` | 41 | Microsoft-Windows-Kernel-Power | Critical | System rebooted without clean shutdown (companion to 6008) |
| `win.tcpip.port_exhaustion` | 4266, 4231 | Tcpip | Warning | Ephemeral port exhaustion — causes silent app/network failures |
| `win.app.crash` | 1000 | Application Error | Warning | Application faulted (crash with module info) |
| `win.app.hang` | 1002 | Application Hang | Warning | Application stopped responding |
| `win.update.install_failure` | 20 | Microsoft-Windows-WindowsUpdateClient | Warning | Windows Update failed to install silently |
| `win.volsnap.shadow_copy_failed` | 36 | Volsnap | Warning | Shadow copies aborted — backup/restore silently broken |
| `win.service.dependency_failure` | 7001 | Service Control Manager | Warning | Service won't start due to dependency failure |

**Tier 2 — Useful but noisier:**

| Rule ID | Event ID | Source | Severity | What it catches |
|---|---|---|---|---|
| `win.vss.error` | 8193 | VSS | Warning | Volume Shadow Copy Service error — backup infra failing |
| `win.dns.timeout` | 1014 | Microsoft-Windows-DNS-Client | Info | DNS resolution timeout (useful in bursts, noisy individually) |
| `win.dotnet.unhandled_exception` | 1026 | .NET Runtime | Warning | .NET app terminated by unhandled exception |

### Milestones

#### E: Historical scan (`telltale scan`)
- Add `scan` subcommand: one-shot read of recent Event Log history (default 48h, configurable)
- Reuse existing Engine for matching + dedup
- For Windows: use `EvtQuery` (pull/historical) instead of `EvtSubscribe` (push/live)
- Output: same colored terminal format as daemon, plus summary ("scanned N events, M alerts found")
- Persist scan results to SQLite like daemon alerts
- This is both a user feature and a dev testing tool

#### F: Expanded rules + real event fixtures
- Add Tier 1 rules (7 new rules)
- Add Tier 2 rules (3 new rules)
- For each rule: capture a real event from the test machine as a JSON fixture
- Unit tests load fixtures and assert rule matches
- New test infrastructure: `fixtures/` directory with captured events, helper to deserialize into `Event`

#### G: Event capture/replay harness
- `telltale capture --duration 60 --output events.json` — record raw events for a time window
- `telltale replay --input events.json` — feed captured events through the engine
- Fixture format: JSON array of serialized `Event` structs
- This closes the loop: capture on real machine → commit fixture → CI tests against real data

### v0.2 Release Gate — COMPLETE
All of the following are complete:
- `telltale scan` working on Windows with configurable time window
- 15 Windows rules with real event test fixtures
- Capture and replay commands functional
- All 26 tests passing

---

## v0.3 — Tauri GUI

### Context
CLI commands (`scan`, `recent`, `rules`, `daemon`) cover power users. A desktop GUI makes telltale accessible as an install-and-forget dashboard — same core engine and SQLite DB, visual presentation layer on top.

### Goals
- Tauri v2 desktop app as a new workspace member
- Dashboard with alert summary, recent alerts, scan trigger
- Alert list with severity filtering, detail view with descriptions + recommended actions
- Rules reference view
- Reads from the same SQLite DB the CLI daemon writes to

### Non-goals (deferred)
- System tray / daemon lifecycle management from GUI
- Auto-start on boot
- Notification preferences UI
- Theme toggle
- macOS / Linux GUI testing (Windows-first)
- Auto-update mechanism

### Architecture

```
crates/telltale-gui/
├── Cargo.toml
├── src/
│   ├── main.rs             # Tauri entry point
│   └── commands.rs         # Tauri IPC commands (thin wrappers around telltale-core)
├── ui/                     # Svelte 5 frontend
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── main.ts
│       ├── App.svelte
│       └── lib/
│           ├── Dashboard.svelte
│           ├── AlertList.svelte
│           ├── AlertDetail.svelte
│           ├── RulesList.svelte
│           └── ScanPanel.svelte
└── tauri.conf.json
```

### Tauri IPC commands

Thin wrappers over `telltale-core`. No business logic in the GUI crate.

| Command | Maps to |
|---|---|
| `get_status()` | DB exists, rule count, checkpoint, alert count |
| `get_recent_alerts(limit, severity?)` | `Store::get_recent()` |
| `get_all_alerts()` | `Store::get_all_alerts()` |
| `get_alert_counts()` | Count by severity (new Store query) |
| `get_rules()` | `knowledge::windows_rules()` / `linux_rules()` |
| `run_scan(hours, severity?)` | Reuses scan logic from CLI |

### Frontend views

**Dashboard** (landing page):
- Summary cards: Critical / Warning / Info counts
- Recent alerts (last 10)
- Quick-scan button ("Scan last 48h")
- Last checkpoint timestamp

**Alerts** (full list):
- Sortable table: severity, title, fingerprint, last seen, count
- Severity filter tabs (All / Critical / Warning / Info)
- Click row → detail panel

**Alert Detail** (slide-out or modal):
- Title, severity badge, rule ID
- Description + recommended action
- First seen / last seen / occurrence count
- Fingerprint entity

**Rules** (reference):
- Table of all loaded rules with severity, title, description

### Tech choices
- **Tauri v2** — current stable, better IPC, smaller bundles
- **Svelte 5** — Tauri default template, minimal JS
- **Vite** — bundler (Tauri default)
- **No CSS framework** — plain CSS, simple dashboard layout

### Dev workflow
- Develop in WSL, test on Windows
- `cargo tauri dev` for hot-reload (WSLg or X11)
- Build Windows installer with `cargo tauri build --target x86_64-pc-windows-msvc` or native build via sync script
- GUI and CLI share the same SQLite DB path via `telltale-core`

### Milestones

#### H: Tauri scaffold + dashboard
- New `telltale-gui` workspace member with Tauri v2 + Svelte 5
- Implement `get_status()`, `get_recent_alerts()`, `get_alert_counts()` commands
- Dashboard view with summary cards + recent alerts
- Builds and runs on Windows

#### I: Alert list + detail + rules views
- Full alert list with severity filtering and sorting
- Alert detail view
- Rules reference page
- Navigation between views

#### J: Scan integration + polish
- Scan trigger from GUI with progress feedback
- Human-readable timestamp formatting
- Empty states, error handling
- Window title, icon

### v0.3 Release Gate
- Tauri app builds and runs on Windows
- Dashboard shows alert summary and recent alerts
- Full alert list with severity filtering
- Alert detail with description + recommended action
- Rules reference view
- Scan triggerable from GUI
- Reads from same SQLite DB as CLI daemon
