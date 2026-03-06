<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  import AlertDetail from './AlertDetail.svelte';

  const FILTERS = ['all', 'critical', 'warning', 'info'];
  const SORT_OPTIONS = ['last_seen', 'occurrence_count', 'severity'];

  let alerts = $state([]);
  let loading = $state(true);
  let error = $state('');
  let filter = $state('all');
  let sortBy = $state('last_seen');
  let sortDirection = $state('desc');
  let selectedAlert = $state(null);

  onMount(() => {
    void loadAlerts();
  });

  const sortedAlerts = $derived.by(() => {
    const list = [...alerts];

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

  function formatTimestamp(epochSeconds) {
    if (!epochSeconds) {
      return 'never';
    }

    return new Date(epochSeconds * 1000).toLocaleString();
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

  function labelForFilter(value) {
    if (value === 'all') {
      return 'All';
    }

    return value[0].toUpperCase() + value.slice(1);
  }

  function labelForSort(value) {
    if (value === 'last_seen') {
      return 'Last Seen';
    }

    if (value === 'occurrence_count') {
      return 'Count';
    }

    return 'Severity';
  }
</script>

<section class="panel alert-list-panel">
  <div class="panel-header">
    <h2>Alert List</h2>
    <button type="button" onclick={loadAlerts} disabled={loading}>
      {loading ? 'Loading...' : 'Refresh'}
    </button>
  </div>

  <div class="panel-toolbar">
    <div class="filter-tabs">
      {#each FILTERS as option}
        <button
          type="button"
          class={`filter-tab ${filter === option ? 'active' : ''}`}
          onclick={() => setFilter(option)}
        >
          {labelForFilter(option)}
        </button>
      {/each}
    </div>

    <div class="sort-controls">
      {#each SORT_OPTIONS as option}
        <button
          type="button"
          class={`sort-button ${sortBy === option ? 'active' : ''}`}
          onclick={() => setSortBy(option)}
        >
          {labelForSort(option)}
        </button>
      {/each}
      <button type="button" class="sort-button" onclick={toggleSortDirection}>
        {sortDirection === 'desc' ? 'Desc' : 'Asc'}
      </button>
    </div>
  </div>

  {#if error}
    <p class="empty">Failed to load alerts: {error}</p>
  {:else if sortedAlerts.length === 0}
    <p class="empty">No alerts found.</p>
  {:else}
    <div class="alerts-table alerts-table-full">
      <div class="table-header">Severity</div>
      <div class="table-header">Title</div>
      <div class="table-header">Fingerprint</div>
      <div class="table-header">Last Seen</div>
      <div class="table-header">Count</div>

      {#each sortedAlerts as alert}
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => openAlertDetail(alert)}>
            <span class={`badge ${alert.severity}`}>{alert.severity}</span>
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => openAlertDetail(alert)}>
            {alert.title}
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button mono" onclick={() => openAlertDetail(alert)}>
            {alert.fingerprint || '(none)'}
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => openAlertDetail(alert)}>
            {formatTimestamp(alert.last_seen)}
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => openAlertDetail(alert)}>
            {alert.occurrence_count}
          </button>
        </div>
      {/each}
    </div>
  {/if}
</section>

{#if selectedAlert}
  <AlertDetail alert={selectedAlert} onClose={closeAlertDetail} />
{/if}
