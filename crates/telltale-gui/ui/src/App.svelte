<svelte:options runes={true} />

<script>
  import AlertList from './lib/AlertList.svelte';
  import Dashboard from './lib/Dashboard.svelte';
  import RulesList from './lib/RulesList.svelte';

  const VIEWS = {
    dashboard: 'Dashboard',
    alerts: 'Alerts',
    rules: 'Rules'
  };

  const VIEW_ICON_PATHS = {
    dashboard: ['M3 10.5 12 3l9 7.5', 'M5 9.5V20h14V9.5', 'M10 20v-6h4v6'],
    alerts: ['M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z', 'M12 9v4', 'M12 17h.01'],
    rules: ['M4 6h16', 'M4 12h16', 'M4 18h16', 'M8 6m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0', 'M14 12m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0', 'M10 18m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0']
  };

  const SIDEBAR_TOGGLE_ICON_PATHS = {
    expanded: ['M3 4h18v16H3z', 'M9 4v16', 'M15 9l-3 3 3 3'],
    collapsed: ['M3 4h18v16H3z', 'M9 4v16', 'M13 9l3 3-3 3']
  };

  let activeView = $state('dashboard');
  let sidebarCollapsed = $state(false);

  function setActiveView(view) {
    activeView = view;
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
  }
</script>

<main class="app-shell">
  <aside class="app-sidebar" class:collapsed={sidebarCollapsed}>
    <button
      type="button"
      class="sidebar-toggle"
      onclick={toggleSidebar}
      aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <svg class="sidebar-toggle-icon" viewBox="0 0 24 24" aria-hidden="true">
        {#each SIDEBAR_TOGGLE_ICON_PATHS[sidebarCollapsed ? 'collapsed' : 'expanded'] as path}
          <path d={path}></path>
        {/each}
      </svg>
    </button>

    <nav class="sidebar-nav" aria-label="Views">
      {#each Object.entries(VIEWS) as [id, label]}
        <button
          type="button"
          class="sidebar-nav-item"
          class:active={activeView === id}
          aria-pressed={activeView === id}
          onclick={() => setActiveView(id)}
          title={sidebarCollapsed ? label : undefined}
        >
          <svg class="nav-item-icon" viewBox="0 0 24 24" aria-hidden="true">
            {#each VIEW_ICON_PATHS[id] as path}
              <path d={path}></path>
            {/each}
          </svg>
          {#if !sidebarCollapsed}
            <span class="nav-item-label">{label}</span>
          {/if}
        </button>
      {/each}
    </nav>
  </aside>

  <section class="app-main-frame">
    <section class="app-main">
      <section class="workspace">
        <section class={`view-container view-container-${activeView}`}>
          {#if activeView === 'dashboard'}
            <Dashboard />
          {:else if activeView === 'alerts'}
            <AlertList />
          {:else}
            <RulesList />
          {/if}
        </section>
      </section>
    </section>
  </section>
</main>
