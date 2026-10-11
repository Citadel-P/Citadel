export const STATS_PRESETS = [
  { minutes: 15, label: '15 minutes' },
  { minutes: 60, label: '1 hour' },
  { minutes: 360, label: '6 hours' },
  { minutes: 1440, label: '24 hours' },
  { minutes: 2880, label: '48 hours' },
  { minutes: 10080, label: '7 days' },
] as const;
export type StatsRange = { kind: 'relative'; minutes: number } | { kind: 'today' | 'yesterday' };
export type StatsBounds = { from: number; to: number };
export type StatsRangeQuery = { hours?: 48; minutes?: number; from?: number; to?: number };

export function resolveStatsRange(range: StatsRange, now = Math.floor(Date.now() / 1000)): StatsBounds {
  if (range.kind === 'relative') return { from: now - range.minutes * 60, to: now };
  const midnight = new Date(now * 1000);
  midnight.setHours(0, 0, 0, 0);
  const today = midnight.getTime() / 1000;
  if (range.kind === 'today') return { from: today, to: now };
  midnight.setDate(midnight.getDate() - 1);
  return { from: midnight.getTime() / 1000, to: today - 1 };
}
export function statsRangeQuery(range: StatsRange, bounds: StatsBounds): StatsRangeQuery {
  if (range.kind === 'relative') {
    // Reuse the supported 48-hour API window.
    return range.minutes === 2880 ? { hours: 48 } : { minutes: range.minutes };
  }
  if (range.kind === 'today') return { from: bounds.from };
  return { from: bounds.from, to: bounds.to };
}
export const statsRangeIsLive = (range: StatsRange) => range.kind === 'relative' || range.kind === 'today';
export function statsBucketSeconds(bounds: StatsBounds) {
  const seconds = bounds.to - bounds.from;
  return seconds <= 3600 ? 10 : seconds <= 21600 ? 30 : seconds <= 86400 ? 60 : seconds <= 259200 ? 300 : 900;
}
export function filterStatsRange<T extends { created?: string | number | null }>(
  stats: readonly T[],
  bounds: StatsBounds,
): T[] {
  return stats.filter((stat) => Number(stat.created) >= bounds.from && Number(stat.created) <= bounds.to);
}
export const statsAxis = (bounds: StatsBounds | undefined) => ({
  tick: { fontSize: 10 },
  ...(bounds
    ? {
        type: 'number' as const,
        domain: [bounds.from, bounds.to] as [number, number],
        allowDataOverflow: true,
      }
    : {}),
});
