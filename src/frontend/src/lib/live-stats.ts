const MAX_STATS_WINDOW_HOURS = 72;
const MONITORING_INTERVAL_SECONDS = 10;
const MAX_RENDERED_STATS_POINTS = 2048;
const LIVE_STATS_COMPACTION_RATIO = 0.75;

export const MAX_LIVE_STATS_POINTS = MAX_RENDERED_STATS_POINTS;
export const getLiveStatsPointLimit = (windowHours: number) =>
  Math.min(
    Math.ceil((Math.min(windowHours, MAX_STATS_WINDOW_HOURS) * 60 * 60) / MONITORING_INTERVAL_SECONDS),
    MAX_RENDERED_STATS_POINTS,
  );

export const STREAMED_STATS_QUERY_OPTIONS = {
  refetchOnReconnect: true,
} as const;

type TimestampedStat = {
  created?: string | number | null;
};

export const appendBoundedLiveStat = <T extends TimestampedStat>(
  current: readonly T[],
  next: T,
  maxPoints = MAX_LIVE_STATS_POINTS,
  maxAgeSeconds = MAX_STATS_WINDOW_HOURS * 60 * 60,
): T[] => {
  if (maxPoints <= 0) return [];

  const lastIndex = current.length - 1;
  const nextTimestamp = toEpochSeconds(next.created);
  if (lastIndex >= 0 && toEpochSeconds(current[lastIndex].created) === nextTimestamp) {
    const updated = current.slice();
    updated[lastIndex] = next;
    return updated;
  }

  if (maxPoints === 1) return [next];
  if (current.length < maxPoints) return [...current, next];

  const candidate = [...current, next];
  const withinWindow =
    nextTimestamp !== null && nextTimestamp > 0 && maxAgeSeconds > 0
      ? candidate.filter((stat) => {
          const timestamp = toEpochSeconds(stat.created);
          return timestamp === null || timestamp >= nextTimestamp - maxAgeSeconds;
        })
      : candidate;

  if (withinWindow.length <= maxPoints) return withinWindow;

  const compactedPointCount = Math.max(2, Math.floor(maxPoints * LIVE_STATS_COMPACTION_RATIO));
  return sampleAcrossTime(withinWindow, compactedPointCount);
};

const toEpochSeconds = (value: TimestampedStat['created']): number | null => {
  if (value === null || value === undefined) return null;

  const numeric = Number(value);
  if (Number.isFinite(numeric)) return numeric;

  const parsed = typeof value === 'string' ? Date.parse(value) : Number.NaN;
  return Number.isFinite(parsed) ? parsed / 1000 : null;
};

const sampleAcrossTime = <T extends TimestampedStat>(values: readonly T[], count: number): T[] => {
  if (values.length <= count) return [...values];
  if (count <= 1) return [values[values.length - 1]];

  const timestamps = values.map((value) => toEpochSeconds(value.created));
  const firstTimestamp = timestamps[0];
  const lastTimestamp = timestamps[timestamps.length - 1];
  if (firstTimestamp === null || lastTimestamp === null || lastTimestamp <= firstTimestamp) {
    return sampleEvenly(values, count);
  }

  const bucketWidth = (lastTimestamp - firstTimestamp) / (count - 1);
  const buckets = new Map<number, T>();
  values.forEach((value, index) => {
    const timestamp = timestamps[index];
    if (timestamp === null) return;

    const bucket = Math.min(count - 1, Math.floor((timestamp - firstTimestamp) / bucketWidth));
    buckets.set(bucket, value);
  });
  buckets.set(0, values[0]);
  buckets.set(count - 1, values[values.length - 1]);
  return [...buckets.entries()]
    .sort(([left], [right]) => left - right)
    .map(([, value]) => value);
};

const sampleEvenly = <T>(values: readonly T[], count: number): T[] => {
  const lastIndex = values.length - 1;
  const sampled = new Array<T>(count);
  for (let index = 0; index < count; index++) {
    sampled[index] = values[Math.round((index * lastIndex) / (count - 1))];
  }
  return sampled;
};

export const mergeStatsByCreated = <T extends TimestampedStat>(historical: readonly T[], live: readonly T[]): T[] => {
  const merged: T[] = [];
  let historicalIndex = 0;
  let liveIndex = 0;

  while (historicalIndex < historical.length && liveIndex < live.length) {
    const historicalStat = historical[historicalIndex];
    const liveStat = live[liveIndex];
    const historicalCreated = Number(historicalStat.created ?? 0);
    const liveCreated = Number(liveStat.created ?? 0);

    if (historicalCreated < liveCreated) {
      merged.push(historicalStat);
      historicalIndex++;
    } else if (historicalCreated > liveCreated) {
      merged.push(liveStat);
      liveIndex++;
    } else {
      merged.push(liveStat);
      historicalIndex++;
      liveIndex++;
    }
  }

  while (historicalIndex < historical.length) {
    merged.push(historical[historicalIndex++]);
  }

  while (liveIndex < live.length) {
    merged.push(live[liveIndex++]);
  }

  return merged;
};
