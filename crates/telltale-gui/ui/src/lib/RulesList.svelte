<svelte:options runes={true} />

<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let rules = $state([]);
  let loading = $state(true);
  let error = $state('');
  let expandedRuleId = $state(null);

  onMount(() => {
    void loadRules();
  });

  async function loadRules() {
    loading = true;
    error = '';

    try {
      const response = await invoke('get_rules');
      rules = response;

      if (expandedRuleId) {
        const exists = response.some((rule) => rule.id === expandedRuleId);
        if (!exists) {
          expandedRuleId = null;
        }
      }
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function toggleExpanded(ruleId) {
    expandedRuleId = expandedRuleId === ruleId ? null : ruleId;
  }
</script>

<section class="panel rules-panel">
  <div class="panel-header">
    <h2>Rules Reference</h2>
    <button type="button" onclick={loadRules} disabled={loading}>
      {loading ? 'Loading...' : 'Refresh'}
    </button>
  </div>

  {#if error}
    <p class="empty">Failed to load rules: {error}</p>
  {:else if rules.length === 0}
    <p class="empty">No rules loaded for this OS.</p>
  {:else}
    <div class="rules-table">
      <div class="table-header">Severity</div>
      <div class="table-header">Rule ID</div>
      <div class="table-header">Title</div>
      <div class="table-header">Description</div>

      {#each rules as rule}
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => toggleExpanded(rule.id)}>
            <span class={`badge ${rule.severity}`}>{rule.severity}</span>
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button mono" onclick={() => toggleExpanded(rule.id)}>
            {rule.id}
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => toggleExpanded(rule.id)}>
            {rule.title}
          </button>
        </div>
        <div class="cell">
          <button type="button" class="cell-button" onclick={() => toggleExpanded(rule.id)}>
            {rule.description}
          </button>
        </div>

        {#if expandedRuleId === rule.id}
          <div class="rule-expanded">
            <p>
              <strong>Recommended action:</strong>
              {rule.recommended_action}
            </p>
            <p>
              <strong>Cooldown:</strong>
              {rule.cooldown_secs} seconds
            </p>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</section>
