<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  import AlertDetail from './AlertDetail.svelte';
  import {
    formatCheckpoint,
    formatFingerprint,
    formatRelativeTime,
    formatTimestamp
  } from './format.js';

  const EMPTY_OVERVIEW = {
    db_exists: false,
    db_path: 'unknown',
    rules_loaded: 0,
    total_alerts: 0,
    last_checkpoint: null,
    last_alert_at: null,
    health_state: 'empty',
    counts: { critical: 0, warning: 0, info: 0 },
    recent_alerts: []
  };

  const HEALTH_COPY = {
    critical: {
      label: 'Needs attention',
      title: 'Critical conditions are active in the event stream.',
      body: 'Start with the newest hardware, filesystem, or shutdown failures and work backward from the freshest signal.'
    },
    warning: {
      label: 'Watch closely',
      title: 'Warning-level failures are present and worth triaging.',
      body: 'The system is not quiet. Review recent warnings before they become noisier or user-visible.'
    },
    quiet: {
      label: 'Quiet',
      title: 'Only informational signal is currently tracked.',
      body: 'The rule pack is active, the database is healthy, and there are no warning or critical conditions in the current local view.'
    },
    empty: {
      label: 'No signal yet',
      title: 'No alert history has been captured yet.',
      body: 'Run a historical scan to populate the local database and establish a baseline for this machine.'
    }
  };

  let overview = $state(EMPTY_OVERVIEW);
  let loading = $state(true);
  let error = $state('');
  let scanHours = $state(48);
  let scanning = $state(false);
  let scanResult = $state(null);
  let scanError = $state('');
  let selectedAlert = $state(null);

  onMount(() => {
    void load();
  });

  const healthCopy = $derived.by(() => HEALTH_COPY[overview.health_state] ?? HEALTH_COPY.empty);
  const lastSignalLabel = $derived.by(() => formatRelativeTime(overview.last_alert_at));
  const checkpointLabel = $derived.by(() => {
    const epochSeconds = Number(overview.last_checkpoint);
    if (Number.isFinite(epochSeconds)) {
      return formatRelativeTime(epochSeconds);
    }

    return overview.last_checkpoint || 'No checkpoint';
  });
  const metricTiles = $derived.by(() => [
    {
      label: 'Critical',
      value: overview.counts.critical,
      tone: 'critical',
      meta: overview.counts.critical > 0 ? 'Active issues' : 'Clear'
    },
    {
      label: 'Warning',
      value: overview.counts.warning,
      tone: 'warning',
      meta: overview.counts.warning > 0 ? 'Needs review' : 'Clear'
    },
    {
      label: 'Info',
      value: overview.counts.info,
      tone: 'info',
      meta: 'Low-severity context'
    },
    {
      label: 'Tracked alerts',
      value: overview.total_alerts,
      tone: 'neutral',
      meta: 'Local SQLite history'
    },
    {
      label: 'Rules loaded',
      value: overview.rules_loaded,
      tone: 'neutral',
      meta: 'Current rule pack'
    },
    {
      label: 'Checkpoint',
      value: checkpointLabel,
      tone: 'neutral',
      meta: formatCheckpoint(overview.last_checkpoint)
    }
  ]);

  async function load() {
    loading = true;
    error = '';

    try {
      const response = await invoke('get_dashboard_overview');
      overview = response;

      if (selectedAlert) {
        selectedAlert = response.recent_alerts.find((item) => item.id === selectedAlert.id) ?? null;
      }
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

  function openAlertDetail(alert) {
    selectedAlert = alert;
  }

  function closeAlertDetail() {
    selectedAlert = null;
  }
</script>

<section class="dashboard view-page dashboard-page">
  <header class="view-header view-header-compact">
    <div>
      <p class="view-kicker">Overview</p>
      <h2>System state at a glance</h2>
      <p class="view-copy">Current signal, fast scan controls, and the latest alerts in one workspace.</p>
    </div>
    <button type="button" onclick={load} disabled={loading}>
      {loading ? 'Refreshing...' : 'Refresh'}
    </button>
  </header>

  {#if error}
    <section class="status-callout status-callout-error">
      <div>
        <p class="callout-label">Dashboard unavailable</p>
        <p class="callout-copy">Failed to load dashboard data: {error}</p>
      </div>
    </section>
  {/if}

  <div class="dashboard-layout">
    <section class={`panel hero-panel hero-panel-${overview.health_state}`}>
      <div class="hero-head">
        <div>
          <p class="callout-label">System health</p>
          <h3>{healthCopy.label}</h3>
        </div>
        <span class={`health-pill health-pill-${overview.health_state}`}>{healthCopy.label}</span>
      </div>

      <div class="hero-body">
        <div class="hero-copy-block">
          <p class="hero-title">{healthCopy.title}</p>
          <p class="hero-copy">{healthCopy.body}</p>
        </div>

        <div class="hero-figure">
          <strong>{overview.total_alerts}</strong>
          <span>tracked alerts</span>
        </div>
      </div>

      <div class="hero-meta-row">
        <span class="meta-chip">Last alert {lastSignalLabel}</span>
        <span class="meta-chip">Rules {overview.rules_loaded}</span>
        <span class="meta-chip">Checkpoint {checkpointLabel}</span>
      </div>
    </section>

    <div class="dashboard-side">
      <section class="panel action-panel compact-panel">
        <div class="section-heading compact-heading">
          <div>
            <p class="section-kicker">Historical scan</p>
            <h3>Run a focused pass</h3>
          </div>
        </div>

        <div class="scan-inline-copy">
          <p class="section-copy">Replay recent OS events through the active rule pack.</p>
        </div>

        <div class="scan-controls">
          <label class="scan-input">
            <span>Hours</span>
            <input type="number" min="0" step="1" bind:value={scanHours} disabled={scanning} />
          </label>

          <button type="button" class="scan-button" onclick={runScan} disabled={scanning}>
            {scanning ? 'Scanning...' : 'Run Scan'}
          </button>
        </div>

        {#if scanError}
          <div class="status-callout status-callout-error compact-callout">
            <div>
              <p class="callout-label">Scan failed</p>
              <p class="callout-copy">{scanError}</p>
            </div>
          </div>
        {/if}

        {#if scanResult}
          <div class="status-callout status-callout-success compact-callout">
            <div>
              <p class="callout-label">Latest scan</p>
              <p class="callout-copy">
                {scanResult.events_scanned} events scanned, {scanResult.alerts_found} visible alerts, {scanResult.new_alerts} new records.
              </p>
            </div>
          </div>
        {/if}
      </section>

      <section class="metric-grid metric-grid-dense">
        {#each metricTiles as tile}
          <article class={`metric-tile metric-tile-${tile.tone}`}>
            <p>{tile.label}</p>
            <strong>{tile.value}</strong>
            <span>{tile.meta}</span>
          </article>
        {/each}
      </section>
    </div>
  </div>

  <section class="panel dashboard-feed-panel">
    <div class="section-heading compact-heading">
      <div>
        <p class="section-kicker">Recent signal</p>
        <h3>Newest alerts</h3>
      </div>
      <span class="section-meta">{overview.recent_alerts.length} shown</span>
    </div>

    {#if loading && overview.recent_alerts.length === 0}
      <div class="empty-state">
        <p class="empty-title">Loading dashboard</p>
        <p class="empty-copy">Collecting the latest status, counts, and recent alerts.</p>
      </div>
    {:else if !overview.db_exists}
      <div class="empty-state">
        <p class="empty-title">No local alert history yet</p>
        <p class="empty-copy">Run a scan to build the first baseline and populate the recent feed.</p>
      </div>
    {:else if overview.recent_alerts.length === 0}
      <div class="empty-state">
        <p class="empty-title">No recent alerts</p>
        <p class="empty-copy">The database is present, but nothing recent matched the current rule set.</p>
      </div>
    {:else}
      <div class="dashboard-alert-list">
        {#each overview.recent_alerts as alert}
          <article class={`dashboard-alert-row dashboard-alert-row-${alert.severity}`}>
            <button type="button" class="dashboard-alert-button" onclick={() => openAlertDetail(alert)}>
              <div class="dashboard-alert-main">
                <div class="dashboard-alert-titlebar">
                  <span class={`badge ${alert.severity}`}>{alert.severity}</span>
                  <h4>{alert.title}</h4>
                </div>
                <p class="dashboard-alert-description">{alert.description}</p>
                <div class="meta-chip-row">
                  <span class="meta-chip mono">{formatFingerprint(alert.fingerprint)}</span>
                  <span class="meta-chip mono">{alert.rule_id}</span>
                  <span class="meta-chip">{alert.occurrence_count} occurrences</span>
                </div>
              </div>

              <div class="dashboard-alert-side">
                <span class="subtle-meta">{formatRelativeTime(alert.last_seen)}</span>
                <span class="subtle-meta">{formatTimestamp(alert.last_seen)}</span>
                <span class="card-action-link">Open details</span>
              </div>
            </button>
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <footer class="status-footer status-footer-dashboard">
    <span>Rules loaded: {overview.rules_loaded}</span>
    <span>Last checkpoint: {formatCheckpoint(overview.last_checkpoint)}</span>
    <span class="mono">DB path: {overview.db_path}</span>
  </footer>
</section>

{#if selectedAlert}
  <AlertDetail alert={selectedAlert} onClose={closeAlertDetail} />
{/if}
