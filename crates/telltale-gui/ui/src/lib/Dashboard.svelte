<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let status = $state(null);
  let counts = $state({ critical: 0, warning: 0, info: 0 });
  let alerts = $state([]);
  let loading = $state(true);
  let error = $state('');

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

  function formatTimestamp(epochSeconds) {
    if (!epochSeconds) {
      return 'never';
    }

    return new Date(epochSeconds * 1000).toLocaleString();
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

  <section class="panel">
    <div class="panel-header">
      <h2>Recent Alerts</h2>
      <button type="button" onclick={load} disabled={loading}>
        {loading ? 'Loading...' : 'Refresh'}
      </button>
    </div>

    {#if error}
      <p class="empty">Failed to load dashboard data: {error}</p>
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
    <span>Last checkpoint: {status?.last_checkpoint ?? 'none'}</span>
    <span class="mono">DB path: {status?.db_path ?? 'unknown'}</span>
  </footer>
</section>
