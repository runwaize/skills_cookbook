/** Renders a relative time string (e.g. "3 days ago") from an RFC3339 timestamp, or a fallback. */
export function relativeTime(iso: string): string {
  if (!iso) return '—';
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '—';
  const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
  const diffMs = then - Date.now();
  const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));
  if (Math.abs(diffDays) >= 1) return rtf.format(diffDays, 'day');
  const diffHours = Math.round(diffMs / (1000 * 60 * 60));
  if (Math.abs(diffHours) >= 1) return rtf.format(diffHours, 'hour');
  const diffMinutes = Math.round(diffMs / (1000 * 60));
  return rtf.format(diffMinutes, 'minute');
}

export function localeDate(iso: string): string {
  if (!iso) return 'unknown';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return 'unknown';
  return d.toLocaleDateString();
}
