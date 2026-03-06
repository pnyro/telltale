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
    <h1>Telltale</h1>
    <nav class="app-nav" aria-label="Views">
      <button
        type="button"
        class={`nav-tab ${activeView === 'dashboard' ? 'active' : ''}`}
        onclick={() => setActiveView('dashboard')}
      >
        Dashboard
      </button>
      <button
        type="button"
        class={`nav-tab ${activeView === 'alerts' ? 'active' : ''}`}
        onclick={() => setActiveView('alerts')}
      >
        Alerts
      </button>
      <button
        type="button"
        class={`nav-tab ${activeView === 'rules' ? 'active' : ''}`}
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
