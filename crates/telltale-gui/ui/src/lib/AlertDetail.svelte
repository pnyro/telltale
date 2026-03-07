<svelte:options runes={true} />

<script>
  import { formatFingerprint, formatRelativeTime, formatTimestamp } from './format.js';

  let { alert, onClose } = $props();

  $effect(() => {
    if (typeof window === 'undefined') {
      return undefined;
    }

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.body.style.overflow = 'hidden';
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<div class="detail-overlay">
  <button
    type="button"
    class="detail-backdrop"
    aria-label="Close alert details"
    onclick={onClose}
  ></button>
  <div
    class={`detail-panel detail-panel-${alert.severity}`}
    role="dialog"
    aria-modal="true"
    aria-label="Alert details"
  >
    <div class="detail-header">
      <div class="detail-title-wrap">
        <div class="detail-badge-row">
          <span class={`badge ${alert.severity}`}>{alert.severity}</span>
          <span class="subtle-meta">{formatRelativeTime(alert.last_seen)}</span>
        </div>
        <h2>{alert.title}</h2>
        <p class="detail-rule mono">Rule ID: {alert.rule_id}</p>
      </div>
      <button type="button" class="detail-close-button" onclick={onClose}>Close</button>
    </div>

    <div class="detail-callout">
      <p class="callout-label">Recommended action</p>
      <p class="callout-copy">{alert.recommended_action}</p>
    </div>

    <section class="detail-section">
      <h3>Description</h3>
      <p class="detail-emphasis">{alert.description}</p>
    </section>

    <dl class="detail-grid">
      <div>
        <dt>Fingerprint</dt>
        <dd class="mono">{formatFingerprint(alert.fingerprint)}</dd>
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
