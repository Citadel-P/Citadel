const MAX_STATS_WINDOW_HOURS = 72;
const MONITORING_INTERVAL_SECONDS = 10;

export const MAX_LIVE_STATS_POINTS = (MAX_STATS_WINDOW_HOURS * 60 * 60) / MONITORING_INTERVAL_SECONDS;
export const getLiveStatsPointLimit = (windowHours: number) =>
  Math.ceil((windowHours * 60 * 60) / MONITORING_INTERVAL_SECONDS);

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
): T[] => {
  if (maxPoints <= 0) return [];

  const lastIndex = current.length - 1;
  if (lastIndex >= 0 && Number(current[lastIndex].created ?? 0) === Number(next.created ?? 0)) {
    const updated = current.slice();
    updated[lastIndex] = next;
    return updated;
  }

  const retained = current.slice(Math.max(0, current.length - maxPoints + 1));
  retained.push(next);
  return retained;
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
