# Rules

Telltale rules live in `telltale-core` and are currently defined in Rust.

- Windows rules: `crates/telltale-core/src/knowledge/windows.rs`
- Linux rules: `crates/telltale-core/src/knowledge/linux.rs`

## Rule structure

Each rule includes:

- `id`: stable unique identifier
- `platform`: target OS
- `severity`: `Critical`, `Warning`, or `Info`
- `title`: short user-facing summary
- `description`: plain-language explanation
- `recommended_action`: immediate action guidance
- `cooldown_secs`: deduplication window
- `match_fn`: event matching logic (`fn(&Event) -> bool`)
- `fingerprint_fn`: entity extractor used in dedup key `(rule_id, fingerprint)`

## Adding a rule

1. Add/extend a matcher function in the relevant platform file.
2. Add/extend a fingerprint function so unrelated entities do not collapse into one alert stream.
3. Add the `Rule` entry to `rules()`.
4. Add tests that assert the matcher is hit for representative events.
5. Run:

```bash
cargo fmt
cargo test --workspace
```

## Current rule inventory

## Windows

| ID | Severity | Detects |
|---|---|---|
| `win.disk.bad_block` | Critical | Disk provider Event ID 7 (bad block report) |
| `win.ntfs.corruption` | Critical | Ntfs provider Event ID 55 (filesystem consistency issue) |
| `win.system.unexpected_shutdown` | Warning | EventLog provider Event ID 6008 (unexpected shutdown) |
| `win.whea.hardware_error` | Critical | WHEA-Logger Event IDs 17-20 (hardware error events) |
| `win.bugcheck.summary` | Critical | BugCheck provider Event ID 1001 (BSOD summary) |

## Linux (dev/experimental)

| ID | Severity | Detects |
|---|---|---|
| `linux.oom_killer` | Critical | Kernel OOM kill messages |
| `linux.ext4_error` | Critical | EXT4 filesystem error messages |
| `linux.auth_failure` | Warning | Authentication failure / failed password patterns |
| `linux.systemd_service_failure` | Warning | systemd unit failure-result patterns |
