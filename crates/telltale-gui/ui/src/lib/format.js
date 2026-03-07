export function formatTimestamp(epochSeconds) {
  if (!epochSeconds) {
    return 'never';
  }

  return new Date(epochSeconds * 1000).toLocaleString();
}

export function formatRelativeTime(epochSeconds) {
  if (!epochSeconds) {
    return 'No recent activity';
  }

  const deltaSeconds = Math.round((Date.now() - epochSeconds * 1000) / 1000);
  const future = deltaSeconds < 0;
  const absolute = Math.abs(deltaSeconds);

  if (absolute < 10) {
    return 'just now';
  }

  const units = [
    { size: 86400, short: 'd' },
    { size: 3600, short: 'h' },
    { size: 60, short: 'm' }
  ];

  for (const unit of units) {
    if (absolute >= unit.size) {
      const value = Math.round(absolute / unit.size);
      return future ? `in ${value}${unit.short}` : `${value}${unit.short} ago`;
    }
  }

  return future ? `in ${absolute}s` : `${absolute}s ago`;
}

export function formatCheckpoint(value) {
  if (!value) {
    return 'none';
  }

  const epochSeconds = Number(value);
  if (!Number.isFinite(epochSeconds)) {
    return value;
  }

  return formatTimestamp(epochSeconds);
}

export function formatFingerprint(fingerprint) {
  return fingerprint || 'General condition';
}

export function formatCooldown(seconds) {
  if (seconds <= 0) {
    return 'No cooldown';
  }

  if (seconds < 60) {
    return `${seconds} ${pluralize(seconds, 'second')}`;
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes} ${pluralize(minutes, 'minute')}`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    const remainingMinutes = minutes % 60;
    if (remainingMinutes === 0) {
      return `${hours} ${pluralize(hours, 'hour')}`;
    }

    return `${hours} ${pluralize(hours, 'hour')} ${remainingMinutes} ${pluralize(remainingMinutes, 'minute')}`;
  }

  const days = Math.floor(hours / 24);
  const remainingHours = hours % 24;
  if (remainingHours === 0) {
    return `${days} ${pluralize(days, 'day')}`;
  }

  return `${days} ${pluralize(days, 'day')} ${remainingHours} ${pluralize(remainingHours, 'hour')}`;
}

function pluralize(value, singular) {
  return value === 1 ? singular : `${singular}s`;
}
