export const STREAMED_STATS_QUERY_OPTIONS = { refetchOnReconnect: true } as const;

type TimestampedStat = { created?: string | number | null };

// Keep the common sorted path linear; reconnects or cached snapshots may arrive out of order.
const orderedStats = <T extends TimestampedStat>(stats: readonly T[]): readonly T[] => {
  const valid = (stat: T) => stat.created != null && Number.isFinite(Number(stat.created));
  if (
    stats.every(
      (stat, index) => valid(stat) && (index === 0 || Number(stats[index - 1].created) < Number(stat.created)),
    )
  )
    return stats;
  const byTime = new Map<number, T>();
  for (const stat of stats) if (valid(stat)) byTime.set(Number(stat.created), stat);
  return [...byTime.values()].sort((a, b) => Number(a.created) - Number(b.created));
};

export const mergeStatsByCreated = <T extends TimestampedStat>(historical: readonly T[], live: readonly T[]): T[] => {
  historical = orderedStats(historical);
  live = orderedStats(live);
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
