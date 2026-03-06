<svelte:options runes={true} />

<script>
  import AlertList from './lib/AlertList.svelte';
  import Dashboard from './lib/Dashboard.svelte';
  import RulesList from './lib/RulesList.svelte';

  let activeView = $state('dashboard');

  function setActiveView(view) {
    activeView = view;
  }
</script>

<main class="app-shell">
  <header class="app-header">
    <div class="brand-block">
      <p class="app-kicker">System Intelligence</p>
      <h1>Telltale</h1>
      <p class="app-subtitle">Proactive visibility for silent OS failures</p>
    </div>

    <nav class="app-nav" aria-label="Views">
      <button
        type="button"
        class={`nav-tab ${activeView === 'dashboard' ? 'active' : ''}`}
        aria-pressed={activeView === 'dashboard'}
        onclick={() => setActiveView('dashboard')}
      >
        Dashboard
      </button>
      <button
        type="button"
        class={`nav-tab ${activeView === 'alerts' ? 'active' : ''}`}
        aria-pressed={activeView === 'alerts'}
        onclick={() => setActiveView('alerts')}
      >
        Alerts
      </button>
      <button
        type="button"
        class={`nav-tab ${activeView === 'rules' ? 'active' : ''}`}
        aria-pressed={activeView === 'rules'}
        onclick={() => setActiveView('rules')}
      >
        Rules
      </button>
    </nav>
  </header>

  <section class="view-container">
    {#if activeView === 'dashboard'}
      <Dashboard />
    {:else if activeView === 'alerts'}
      <AlertList />
    {:else}
      <RulesList />
    {/if}
  </section>
</main>
