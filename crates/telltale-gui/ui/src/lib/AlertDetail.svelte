<svelte:options runes={true} />

<script>
  let { alert, onClose } = $props();

  function formatTimestamp(epochSeconds) {
    if (!epochSeconds) {
      return 'never';
    }

    return new Date(epochSeconds * 1000).toLocaleString();
  }
</script>

<div class="detail-overlay">
  <button
    type="button"
    class="detail-backdrop"
    aria-label="Close alert details"
    onclick={onClose}
  ></button>
  <div
    class="detail-panel"
    role="dialog"
    aria-modal="true"
    aria-label="Alert details"
  >
    <div class="detail-header">
      <div class="detail-title-wrap">
        <h2>{alert.title}</h2>
        <span class={`badge ${alert.severity}`}>{alert.severity}</span>
      </div>
      <button type="button" onclick={onClose}>Close</button>
    </div>

    <p class="detail-rule mono">Rule ID: {alert.rule_id}</p>

    <section class="detail-section">
      <h3>Description</h3>
      <p class="detail-emphasis">{alert.description}</p>
    </section>

    <section class="detail-section">
      <h3>Recommended action</h3>
      <p>{alert.recommended_action}</p>
    </section>

    <dl class="detail-grid">
      <div>
        <dt>Fingerprint</dt>
        <dd class="mono">{alert.fingerprint || '(none)'}</dd>
      </div>
      <div>
        <dt>First seen</dt>
        <dd>{formatTimestamp(alert.first_seen)}</dd>
      </div>
      <div>
        <dt>Last seen</dt>
        <dd>{formatTimestamp(alert.last_seen)}</dd>
      </div>
      <div>
        <dt>Occurrences</dt>
        <dd>{alert.occurrence_count}</dd>
      </div>
    </dl>
  </div>
</div>
