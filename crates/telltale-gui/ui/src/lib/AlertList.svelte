<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  import AlertDetail from './AlertDetail.svelte';
  import { formatFingerprint, formatRelativeTime, formatTimestamp } from './format.js';

  const FILTERS = [
    { id: 'all', label: 'All' },
    { id: 'critical', label: 'Critical' },
    { id: 'warning', label: 'Warning' },
    { id: 'info', label: 'Info' }
  ];

  const SORT_OPTIONS = [
    { id: 'last_seen', label: 'Last seen' },
    { id: 'occurrence_count', label: 'Count' },
    { id: 'severity', label: 'Severity' }
  ];

  let alerts = $state([]);
  let loading = $state(true);
  let error = $state('');
  let filter = $state('all');
  let sortBy = $state('last_seen');
  let sortDirection = $state('desc');
  let searchQuery = $state('');
  let selectedAlert = $state(null);

  onMount(() => {
    void loadAlerts();
  });

  const visibleAlerts = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    const list = alerts.filter((alert) => {
      if (!query) {
        return true;
      }

      return [alert.title, alert.rule_id, alert.fingerprint]
        .some((value) => (value || '').toLowerCase().includes(query));
    });

    list.sort((left, right) => {
      let result = 0;

      if (sortBy === 'occurrence_count') {
        result = left.occurrence_count - right.occurrence_count;
      } else if (sortBy === 'severity') {
        result = severityRank(left.severity) - severityRank(right.severity);
      } else {
        result = left.last_seen - right.last_seen;
      }

      return sortDirection === 'asc' ? result : -result;
    });

    return list;
  });

  async function loadAlerts() {
    loading = true;
    error = '';

    try {
      const severity = filter === 'all' ? null : filter;
      const response = await invoke('get_recent_alerts', { limit: 100, severity });
      alerts = response;

      if (selectedAlert) {
        selectedAlert = response.find((item) => item.id === selectedAlert.id) ?? null;
      }
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function setFilter(nextFilter) {
    if (filter === nextFilter) {
      return;
    }

    filter = nextFilter;
    void loadAlerts();
  }

  function setSortBy(nextSort) {
    if (sortBy === nextSort) {
      toggleSortDirection();
      return;
    }

    sortBy = nextSort;
  }

  function toggleSortDirection() {
    sortDirection = sortDirection === 'desc' ? 'asc' : 'desc';
  }

  function openAlertDetail(alert) {
    selectedAlert = alert;
  }

  function closeAlertDetail() {
    selectedAlert = null;
  }

  function severityRank(severity) {
    if (severity === 'critical') {
      return 0;
    }

    if (severity === 'warning') {
      return 1;
    }

    return 2;
  }
</script>

<section class="alerts-view view-page">
  <header class="view-header">
    <div>
      <p class="view-kicker">Alerts</p>
      <h2>Investigate the live alert catalog</h2>
      <p class="view-copy">
        Filter by severity, search locally across alert metadata, and open the detail drawer when you need the full rule context.
      </p>
    </div>
    <button type="button" onclick={loadAlerts} disabled={loading}>
      {loading ? 'Refreshing...' : 'Refresh Alerts'}
    </button>
  </header>

  <section class="panel toolbar-panel">
    <div class="control-row">
      <div class="segmented-control" aria-label="Severity filters">
        {#each FILTERS as option}
          <button
            type="button"
            class={`segment-button ${filter === option.id ? 'active' : ''}`}
            aria-pressed={filter === option.id}
            onclick={() => setFilter(option.id)}
          >
            {option.label}
          </button>
        {/each}
      </div>

      <div class="segmented-control segmented-control-sort" aria-label="Sort alerts">
        {#each SORT_OPTIONS as option}
          <button
            type="button"
            class={`segment-button ${sortBy === option.id ? 'active' : ''}`}
            aria-pressed={sortBy === option.id}
            onclick={() => setSortBy(option.id)}
          >
            {option.label}
          </button>
        {/each}
        <button type="button" class="segment-button" onclick={toggleSortDirection}>
          {sortDirection === 'desc' ? 'Desc' : 'Asc'}
        </button>
      </div>
    </div>

    <label class="search-field">
      <span>Search alerts</span>
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Filter by title, rule ID, or fingerprint"
      />
    </label>
  </section>

  <section class="panel alert-results-panel">
    <div class="section-heading">
      <div>
        <p class="section-kicker">Result set</p>
        <h3>Alert feed</h3>
      </div>
      <span class="section-meta">{visibleAlerts.length} visible</span>
    </div>

    {#if error}
      <div class="status-callout status-callout-error">
        <div>
          <p class="callout-label">Unable to load alerts</p>
          <p class="callout-copy">{error}</p>
        </div>
      </div>
    {:else if loading && alerts.length === 0}
      <div class="empty-state">
        <p class="empty-title">Loading alerts</p>
        <p class="empty-copy">Fetching the latest alert records from the local database.</p>
      </div>
    {:else if visibleAlerts.length === 0}
      <div class="empty-state">
        <p class="empty-title">No alerts match this view</p>
        <p class="empty-copy">
          {searchQuery ? 'Try a broader search term or different severity filter.' : 'The current severity filter returned no alerts.'}
        </p>
      </div>
    {:else}
      <div class="alert-card-list">
        {#each visibleAlerts as alert}
          <article class={`alert-card severity-${alert.severity}`}>
            <button type="button" class="alert-card-button" onclick={() => openAlertDetail(alert)}>
              <div class="alert-card-top">
                <div class="alert-card-title-group">
                  <span class={`badge ${alert.severity}`}>{alert.severity}</span>
                  <h3>{alert.title}</h3>
                  <p class="alert-card-description">{alert.description}</p>
                </div>

                <div class="alert-card-stats">
                  <span class="subtle-label">Last seen</span>
                  <strong>{formatTimestamp(alert.last_seen)}</strong>
                  <span class="subtle-meta">{formatRelativeTime(alert.last_seen)}</span>
                </div>
              </div>

              <div class="meta-chip-row">
                <span class="meta-chip mono">{formatFingerprint(alert.fingerprint)}</span>
                <span class="meta-chip mono">{alert.rule_id}</span>
                <span class="meta-chip">{alert.occurrence_count} occurrences</span>
                <span class="meta-chip">First seen {formatTimestamp(alert.first_seen)}</span>
              </div>

              <div class="card-action-row">
                <span class="card-action-copy">{alert.recommended_action}</span>
                <span class="card-action-link">Inspect alert</span>
              </div>
            </button>
          </article>
        {/each}
      </div>
    {/if}
  </section>
</section>

{#if selectedAlert}
  <AlertDetail alert={selectedAlert} onClose={closeAlertDetail} />
{/if}
