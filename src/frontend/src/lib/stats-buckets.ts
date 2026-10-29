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
  'diskUsage',
  'diskUsedBytes',
  'diskTotalBytes',
] as const;
type Accumulator<T> = {
  seconds: number;
  latest: T;
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
    sums: sameBucket ? { ...previous?.sums } : {},
    counts: sameBucket ? { ...previous?.counts } : {},
  };
  const duplicate = sameBucket && previous && timestamp === Number(previous.latest.created);
  const point: Bucket<T> = { ...next, created: sameBucket ? last?.created : next.created, [accumulator]: state };
  for (const key of metrics) {
    const value = (next as Record<string, unknown>)[key];
    const oldValue = duplicate ? (previous.latest as Record<string, unknown>)[key] : undefined;
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
    if (key in next || count) (point as Record<string, unknown>)[key] = count ? sum / count : null;
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
