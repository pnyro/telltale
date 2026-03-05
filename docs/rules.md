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
| `win.kernel_power.dirty_reboot` | Critical | Microsoft-Windows-Kernel-Power Event ID 41 (dirty reboot/reset) |
| `win.tcpip.port_exhaustion` | Warning | Tcpip Event IDs 4231/4266 (ephemeral TCP/UDP port exhaustion) |
| `win.app.crash` | Warning | Application Error Event ID 1000 (application crash) |
| `win.app.hang` | Info | Application Hang Event ID 1002 (application not responding) |
| `win.update.install_failure` | Warning | WindowsUpdateClient Event ID 20 (update install failure) |
| `win.volsnap.shadow_copy_failed` | Warning | Volsnap Event ID 36 (shadow copy aborted due to storage limits) |
| `win.service.dependency_failure` | Warning | Service Control Manager Event ID 7001 (dependency startup failure) |
| `win.vss.error` | Warning | VSS Event ID 8193 (Volume Shadow Copy Service error) |
| `win.dns.timeout` | Info | DNS Client Event ID 1014 (DNS resolution timeout) |
| `win.dotnet.unhandled_exception` | Warning | .NET Runtime Event ID 1026 (unhandled exception crash) |

## Linux (dev/experimental)

| ID | Severity | Detects |
|---|---|---|
| `linux.oom_killer` | Critical | Kernel OOM kill messages |
| `linux.ext4_error` | Critical | EXT4 filesystem error messages |
| `linux.auth_failure` | Warning | Authentication failure / failed password patterns |
| `linux.systemd_service_failure` | Warning | systemd unit failure-result patterns |
