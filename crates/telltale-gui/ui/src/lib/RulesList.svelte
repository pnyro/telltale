<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  import { formatCooldown } from './format.js';

  const SEVERITY_GROUPS = [
    {
      id: 'critical',
      label: 'Critical',
      description: 'High-severity failures that should interrupt normal operation and triage.'
    },
    {
      id: 'warning',
      label: 'Warning',
      description: 'Operational issues that need review before they become user-visible failures.'
    },
    {
      id: 'info',
      label: 'Info',
      description: 'Lower-severity signal that adds context without demanding immediate action.'
    }
  ];

  let rules = $state([]);
  let loading = $state(true);
  let error = $state('');

  onMount(() => {
    void loadRules();
  });

  const groupedRules = $derived.by(() => {
    const grouped = {
      critical: [],
      warning: [],
      info: []
    };

    for (const rule of rules) {
      const key = grouped[rule.severity] ? rule.severity : 'info';
      grouped[key].push(rule);
    }

    return grouped;
  });

  async function loadRules() {
    loading = true;
    error = '';

    try {
      rules = await invoke('get_rules');
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }
</script>

<section class="rules-view view-page">
  <header class="view-header">
    <div>
      <p class="view-kicker">Rules</p>
      <h2>Understand the rule pack</h2>
      <p class="view-copy">
        Browse the active detection catalog by severity and inspect what each rule explains, recommends, and how aggressively it deduplicates repeat signal.
      </p>
    </div>
    <button type="button" onclick={loadRules} disabled={loading}>
      {loading ? 'Refreshing...' : 'Refresh Rules'}
    </button>
  </header>

  {#if error}
    <section class="panel status-callout status-callout-error">
      <div>
        <p class="callout-label">Unable to load rules</p>
        <p class="callout-copy">{error}</p>
      </div>
    </section>
  {:else if loading && rules.length === 0}
    <section class="panel empty-state">
      <p class="empty-title">Loading rules</p>
      <p class="empty-copy">Collecting the current rule inventory for this operating system.</p>
    </section>
  {:else if rules.length === 0}
    <section class="panel empty-state">
      <p class="empty-title">No rules loaded</p>
      <p class="empty-copy">This operating system does not currently have a rule pack available.</p>
    </section>
  {:else}
    <div class="rules-board">
      {#each SEVERITY_GROUPS as group}
        <section class={`panel rules-lane rules-lane-${group.id}`}>
          <div class="section-heading section-heading-lane">
            <div>
              <p class="section-kicker">{group.label}</p>
              <h3>{groupedRules[group.id].length} rules</h3>
            </div>
          </div>

          <p class="section-copy">{group.description}</p>

          {#if groupedRules[group.id].length === 0}
            <div class="empty-state empty-state-inline">
              <p class="empty-title">No {group.label.toLowerCase()} rules</p>
              <p class="empty-copy">This severity bucket is empty for the current OS.</p>
            </div>
          {:else}
            <div class="rule-card-list">
              {#each groupedRules[group.id] as rule}
                <article class={`rule-card rule-card-${group.id}`}>
                  <div class="rule-card-head">
                    <span class={`badge ${rule.severity}`}>{rule.severity}</span>
                    <p class="rule-id mono">{rule.id}</p>
                  </div>

                  <h4>{rule.title}</h4>
                  <p class="rule-description">{rule.description}</p>

                  <div class="rule-callout">
                    <p class="callout-label">Recommended action</p>
                    <p class="callout-copy">{rule.recommended_action}</p>
                  </div>

                  <div class="meta-chip-row">
                    <span class="meta-chip">Cooldown {formatCooldown(rule.cooldown_secs)}</span>
                  </div>
                </article>
              {/each}
            </div>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</section>
