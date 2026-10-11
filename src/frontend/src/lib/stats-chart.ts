import { byteTransform } from './bytes.helper';
import { mergeStatsByCreated } from './live-stats';
import { statsBucketSeconds, type StatsBounds } from './stats-range';

/** Empty points break the line when telemetry is missing instead of inventing a trend. */
export function statsChartData<T extends { created?: number | string | null }>(
  stats: readonly T[],
  bounds?: StatsBounds,
) {
  const ordered = mergeStatsByCreated([], stats);
  const gapSeconds = 3 * (bounds ? statsBucketSeconds(bounds) : 10);
  const result: Array<T | { created: number }> = [];
  ordered.forEach((stat, index) => {
    const previous = ordered[index - 1];
    const created = Number(stat.created);
    if (previous && created - Number(previous.created) > gapSeconds) {
      result.push({ created: Number(previous.created) + gapSeconds });
    }
    result.push(stat);
  });
  return result;
}

export const statsValueAxis = (unit: 'percent' | 'bytes') => ({
  domain: [0, 'auto'] as [number, string],
  width: 68,
  tickCount: 5,
  tick: { fontSize: 10 },
  tickLine: false,
  axisLine: false,
  tickMargin: 8,
  tickFormatter: (value: number) =>
    unit === 'percent' ? `${Number(value.toFixed(2))}%` : String(byteTransform(value, 1)),
});
