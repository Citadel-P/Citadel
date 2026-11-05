import { mergeStatsByCreated } from './live-stats';

// Internal sample counts are not API fields or plotted metrics.
const accumulator = Symbol('statsBucket');
const latestObservation = Symbol('latestObservation');
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
  'rxBytesPerSecond',
  'txBytesPerSecond',
  'blockReadBytes',
  'blockWriteBytes',
  'blockReadBytesDelta',
  'blockWriteBytesDelta',
  'blockReadBytesPerSecond',
  'blockWriteBytesPerSecond',
  'diskUsage',
  'diskUsedBytes',
  'diskTotalBytes',
] as const;
type Accumulator<T> = {
  seconds: number;
  latest: T;
  baseline?: T;
  sums: Partial<Record<(typeof metrics)[number], number>>;
  // Gauges count observations; throughput weights by measured elapsed seconds.
  counts: Partial<Record<(typeof metrics)[number], number>>;
};
type Bucket<T> = T & { [accumulator]?: Accumulator<T>; [latestObservation]?: T };

export function latestLiveStat<T extends Stat>(stats: readonly T[]): T | undefined {
  const last = stats.at(-1) as Bucket<T> | undefined;
  return last?.[latestObservation] ?? last?.[accumulator]?.latest ?? last;
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
  const measured = withCounterDeltas(next, state.baseline);
  const previousMeasured = previous ? withCounterDeltas(previous.latest, previous.baseline) : undefined;
  const point: Bucket<T> = { ...next, created: sameBucket ? last?.created : next.created, [accumulator]: state };
  const elapsed = (sample: T, baseline?: T) => {
    const duration = Number(sample.created) - Number(baseline?.created);
    return duration > 0 && Number.isFinite(duration) ? duration : 1;
  };
  for (const key of metrics) {
    const weight = key.endsWith('PerSecond') ? elapsed(next, state.baseline) : 1;
    const oldWeight = key.endsWith('PerSecond') && previous ? elapsed(previous.latest, previous.baseline) : 1;
    const value = (measured as Record<string, unknown>)[key];
    const oldValue = duplicate ? (previousMeasured as Record<string, unknown>)[key] : undefined;
    let sum = state.sums[key] ?? 0;
    let count = state.counts[key] ?? 0;
    if (oldValue != null && Number.isFinite(Number(oldValue))) {
      sum -= Number(oldValue) * oldWeight;
      count -= oldWeight;
    }
    if (value != null && Number.isFinite(Number(value))) {
      sum += Number(value) * weight;
      count += weight;
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

/** Match SQL: aggregate each physical Container in time before summing replicas. */
export function sumStatBuckets<T extends Stat>(series: readonly (readonly T[])[], seconds: number): T[] {
  const byBucket = new Map<number, T[]>();
  for (const container of series) {
    for (const stat of averageStatBuckets(container, seconds)) {
      const key = Math.floor(Number(stat.created) / seconds);
      const group = byBucket.get(key) ?? [];
      group.push(stat);
      byBucket.set(key, group);
    }
  }
  const sum = (samples: readonly T[], created: number): T => {
    const result = { created } as T & Record<string, unknown>;
    for (const metric of metrics) {
      let total: number | null = null;
      for (const sample of samples) {
        const value = (sample as Record<string, unknown>)[metric];
        if (typeof value === 'number' && Number.isFinite(value)) total = (total ?? 0) + value;
      }
      (result as Record<string, unknown>)[metric] = total;
    }
    return result;
  };
  const latest = series.map(latestLiveStat).filter((stat): stat is T => stat !== undefined);
  const current = sum(latest, Math.max(0, ...latest.map((stat) => Number(stat.created))));
  return [...byBucket.entries()]
    .sort(([a], [b]) => a - b)
    .map(([, samples]) => ({
      ...sum(samples, Math.min(...samples.map((stat) => Number(stat.created)))),
      [latestObservation]: current,
    }));
}

/** Convert a raw live observation before averaging; each Container has its own baseline. */
export function withCounterDeltas<T extends Stat>(next: T, previous?: T): T {
  const point = { ...next } as T & Record<string, unknown>;
  const current = next as Record<string, unknown>;
  const before = previous as Record<string, unknown> | undefined;
  for (const field of ['rxBytes', 'txBytes', 'blockReadBytes', 'blockWriteBytes'] as const) {
    const delta = `${field}Delta`;
    const rate = `${field}PerSecond`;
    if (!(field in current) || (delta in current && rate in current)) continue;
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
    const increase = valid ? (value >= baseline ? value - baseline : value) : null;
    if (!(delta in current)) (point as Record<string, unknown>)[delta] = increase;
    if (!(rate in current))
      (point as Record<string, unknown>)[rate] =
        increase == null ? null : increase / (Number(next.created) - Number(previous?.created));
  }
  return point;
}
