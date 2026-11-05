import { mergeStatsByCreated } from './live-stats';

// Internal sample counts are not API fields or plotted metrics.
const accumulator = Symbol('statsBucket');
type Stat = { created?: number | string | null };
const metrics = [
  'cpuUsage',
  'memoryActive',
  'memoryCache',
  'memoryLimit',
  'memoryUsage',
  'rxBytes',
  'txBytes',
  'rxBytesDelta',
  'txBytesDelta',
  'diskUsage',
  'diskUsedBytes',
  'diskTotalBytes',
] as const;
type Accumulator<T> = {
  seconds: number;
  latest: T;
  baseline?: T;
  sums: Partial<Record<(typeof metrics)[number], number>>;
  counts: Partial<Record<(typeof metrics)[number], number>>;
};
type Bucket<T> = T & { [accumulator]?: Accumulator<T> };

export function latestLiveStat<T extends Stat>(stats: readonly T[]): T | undefined {
  const last = stats.at(-1) as Bucket<T> | undefined;
  return last?.[accumulator]?.latest ?? last;
}

/** Retain averages, not a sampled subset of observations, for long-lived tabs. */
export function appendLiveStatBucket<T extends Stat>(
  current: readonly T[],
  next: T,
  seconds: number,
  windowSeconds: number,
): T[] {
  const timestamp = Number(next.created);
  if (!Number.isFinite(timestamp)) return [...current];
  let last = current.at(-1) as Bucket<T> | undefined;
  // A different resolution needs fresh observations; HTTP supplies its history.
  if (last?.[accumulator]?.seconds !== seconds) {
    current = [];
    last = undefined;
  }
  const previous = last?.[accumulator];
  if (previous && timestamp < Number(previous.latest.created)) return [...current];
  const sameBucket = last && Math.floor(Number(last.created) / seconds) === Math.floor(timestamp / seconds);
  const state: Accumulator<T> = {
    seconds,
    latest: next,
    baseline: previous?.latest,
    sums: sameBucket ? { ...previous?.sums } : {},
    counts: sameBucket ? { ...previous?.counts } : {},
  };
  const duplicate = sameBucket && previous && timestamp === Number(previous.latest.created);
  if (duplicate) state.baseline = previous.baseline;
  const measured = withNetworkDelta(next, state.baseline);
  const previousMeasured = previous ? withNetworkDelta(previous.latest, previous.baseline) : undefined;
  const point: Bucket<T> = { ...next, created: sameBucket ? last?.created : next.created, [accumulator]: state };
  for (const key of metrics) {
    const value = (measured as Record<string, unknown>)[key];
    const oldValue = duplicate ? (previousMeasured as Record<string, unknown>)[key] : undefined;
    let sum = state.sums[key] ?? 0;
    let count = state.counts[key] ?? 0;
    if (oldValue != null && Number.isFinite(Number(oldValue))) {
      sum -= Number(oldValue);
      count--;
    }
    if (value != null && Number.isFinite(Number(value))) {
      sum += Number(value);
      count++;
    }
    state.sums[key] = sum;
    state.counts[key] = count;
    if (key in measured || count) {
      (point as Record<string, unknown>)[key] = count ? (key.endsWith('Delta') ? sum : sum / count) : null;
    }
  }
  const retained = (sameBucket ? current.slice(0, -1) : current).filter(
    (stat) => Number(stat.created) >= timestamp - windowSeconds - seconds,
  );
  return [...retained, point];
}

/** SQL averages each metric per epoch-aligned bucket before summing replicas. */
export function averageStatBuckets<T extends Stat>(stats: readonly T[], seconds: number): T[] {
  const groups = new Map<number, T[]>();
  for (const stat of mergeStatsByCreated([], stats)) {
    const metadata = (stat as Bucket<T>)[accumulator];
    if (metadata && metadata.seconds !== seconds) continue;
    const key = Math.floor(Number(stat.created) / seconds);
    const group = groups.get(key) ?? [];
    group.push(stat);
    groups.set(key, group);
  }
  return [...groups.values()].map((group) =>
    group.length === 1
      ? group[0]
      : group.reduce(
          (buckets, stat) => appendLiveStatBucket(buckets, stat, seconds, Number.MAX_SAFE_INTEGER),
          [] as T[],
        )[0],
  );
}

export function mergeStatsBuckets<T extends Stat>(history: readonly T[], live: readonly T[], seconds: number): T[] {
  const byBucket = new Map<number, T>();
  for (const stat of averageStatBuckets(live, seconds)) byBucket.set(Math.floor(Number(stat.created) / seconds), stat);
  // Historical averages have unknown sample counts. Do not average an average
  // with raw samples or replace it with a partial live bucket after joining.
  for (const stat of history) byBucket.set(Math.floor(Number(stat.created) / seconds), stat);
  return [...byBucket.entries()].sort(([a], [b]) => a - b).map(([, stat]) => stat);
}

/** Convert a raw live observation before averaging; each Container has its own baseline. */
export function withNetworkDelta<T extends Stat>(next: T, previous?: T): T {
  const point = { ...next } as T & Record<string, unknown>;
  const current = next as Record<string, unknown>;
  const before = previous as Record<string, unknown> | undefined;
  for (const field of ['rxBytes', 'txBytes'] as const) {
    const delta = `${field}Delta`;
    if (delta in current || !(field in current)) continue;
    const value = current[field];
    const baseline = before?.[field];
    const sameContainer = !current.containerId || !before?.containerId || current.containerId === before.containerId;
    const valid =
      sameContainer &&
      previous &&
      Number(next.created) > Number(previous.created) &&
      typeof value === 'number' &&
      Number.isFinite(value) &&
      value >= 0 &&
      typeof baseline === 'number' &&
      Number.isFinite(baseline) &&
      baseline >= 0;
    (point as Record<string, unknown>)[delta] = valid ? (value >= baseline ? value - baseline : value) : null;
  }
  return point;
}
