<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let status = $state(null);
  let counts = $state({ critical: 0, warning: 0, info: 0 });
  let alerts = $state([]);
  let loading = $state(true);
  let error = $state('');
  let scanHours = $state(48);
  let scanning = $state(false);
  let scanResult = $state(null);
  let scanError = $state('');

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = '';

    try {
      const [statusResponse, countResponse, recentAlerts] = await Promise.all([
        invoke('get_status'),
        invoke('get_alert_counts'),
        invoke('get_recent_alerts', { limit: 10, severity: null })
      ]);

      status = statusResponse;
      counts = countResponse;
      alerts = recentAlerts;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function runScan() {
    scanning = true;
    scanError = '';

    try {
      const parsed = Number(scanHours);
      const hours = Number.isFinite(parsed) ? Math.max(0, Math.floor(parsed)) : 48;
      scanHours = hours;

      scanResult = await invoke('run_scan', { hours, severity: null });
      await load();
    } catch (err) {
      scanError = String(err);
    } finally {
      scanning = false;
    }
  }

  function formatTimestamp(epochSeconds) {
    if (!epochSeconds) {
      return 'never';
    }

    return new Date(epochSeconds * 1000).toLocaleString();
  }

  function formatCheckpoint(value) {
    if (!value) {
      return 'none';
    }

    const epochSeconds = Number(value);
    if (!Number.isFinite(epochSeconds)) {
      return value;
    }

    return formatTimestamp(epochSeconds);
  }
</script>

<section class="dashboard">
  <div class="summary-grid">
    <article class="summary-card critical">
      <h2>Critical</h2>
      <p>{counts.critical}</p>
    </article>
    <article class="summary-card warning">
      <h2>Warning</h2>
      <p>{counts.warning}</p>
    </article>
    <article class="summary-card info">
      <h2>Info</h2>
      <p>{counts.info}</p>
    </article>
  </div>

  <section class="panel scan-panel">
    <div class="panel-header">
      <h2>Scan</h2>
    </div>

    <div class="scan-controls">
      <label class="scan-input">
        <span>Hours</span>
        <input type="number" min="0" step="1" bind:value={scanHours} disabled={scanning} />
      </label>

      <button type="button" onclick={runScan} disabled={scanning}>
        {scanning ? 'Scanning...' : 'Run Scan'}
      </button>
    </div>

    {#if scanError}
      <p class="empty scan-error">Scan failed: {scanError}</p>
    {/if}

    {#if scanResult}
      <p class="scan-result">
        Events scanned: {scanResult.events_scanned} | Alerts found: {scanResult.alerts_found} | New alerts: {scanResult.new_alerts}
      </p>
    {/if}
  </section>

  <section class="panel">
    <div class="panel-header">
      <h2>Recent Alerts</h2>
      <button type="button" onclick={load} disabled={loading}>
        {loading ? 'Loading...' : 'Refresh'}
      </button>
    </div>

    {#if error}
      <p class="empty">Failed to load dashboard data: {error}</p>
    {:else if !status?.db_exists}
      <p class="empty">No data yet. Run a scan to get started.</p>
    {:else if alerts.length === 0}
      <p class="empty">No recent alerts.</p>
    {:else}
      <div class="alerts-table">
        <div class="table-header">Severity</div>
        <div class="table-header">Title</div>
        <div class="table-header">Fingerprint</div>
        <div class="table-header">Last Seen</div>
        <div class="table-header">Count</div>

        {#each alerts as alert}
          <div class="cell">
            <span class={`badge ${alert.severity}`}>{alert.severity}</span>
          </div>
          <div class="cell">{alert.title}</div>
          <div class="cell mono">{alert.fingerprint || '(none)'}</div>
          <div class="cell">{formatTimestamp(alert.last_seen)}</div>
          <div class="cell">{alert.occurrence_count}</div>
        {/each}
      </div>
    {/if}
  </section>

  <footer class="status-footer">
    <span>Rules loaded: {status?.rules_loaded ?? 0}</span>
    <span>Last checkpoint: {formatCheckpoint(status?.last_checkpoint ?? null)}</span>
    <span class="mono">DB path: {status?.db_path ?? 'unknown'}</span>
  </footer>
</section>
