import { statsChartData } from './stats-chart';
import { resolveStatsRange, statsRangeQuery, filterStatsRange, statsBucketSeconds } from './stats-range';
import { appendBoundedLiveStat, mergeStatsByCreated } from './live-stats';

describe('live statistics retention', () => {
  it('compacts samples while preserving the oldest and newest timestamps', () => {
    const stats = [1, 2, 3].reduce(
      (current, created) => appendBoundedLiveStat(current, { created }, 2),
      [] as Array<{ created: number }>,
    );

    expect(stats).toEqual([{ created: 1 }, { created: 3 }]);
  });

  it('keeps a bounded representation of the full live window', () => {
    const maxPoints = 20;
    const windowSeconds = 60 * 60;
    const stats = Array.from({ length: 500 }, (_, index) => index * 10).reduce(
      (current, created) => appendBoundedLiveStat(current, { created }, maxPoints, windowSeconds),
      [] as Array<{ created: number }>,
    );

    expect(stats.length).toBeLessThanOrEqual(maxPoints);
    expect(stats[0].created).toBeGreaterThanOrEqual(4990 - windowSeconds - 60);
    expect(stats[0].created).toBeLessThanOrEqual(4990 - windowSeconds + 300);
    expect(stats.at(-1)).toEqual({ created: 4990 });
  });

  it('replaces a sample with the same timestamp', () => {
    const original = [{ created: 1, value: 10 }];
    const updated = appendBoundedLiveStat(original, { created: 1, value: 20 });

    expect(updated).toEqual([{ created: 1, value: 20 }]);
    expect(original).toEqual([{ created: 1, value: 10 }]);
  });

  it('linearly merges sorted history and live samples while preferring live duplicates', () => {
    const now = new Date(2026, 9, 11, 14, 30).getTime() / 1000;
    const relative = resolveStatsRange({ kind: 'relative', minutes: 15 }, now);
    expect(relative).toEqual({ from: now - 900, to: now });
    expect(statsBucketSeconds(relative)).toBe(10);
    expect(statsBucketSeconds({ from: now - 7 * 86400, to: now })).toBe(900);
    const custom = { from: now - 7200, to: now - 3600 };
    expect(
      filterStatsRange(
        [{ created: custom.from - 1 }, { created: custom.from }, { created: custom.to }, { created: now }],
        custom,
      ),
    ).toEqual([{ created: custom.from }, { created: custom.to }]);
    const today = resolveStatsRange({ kind: 'today' }, now);
    expect(new Date(today.from * 1000).getHours()).toBe(0);
    expect(statsRangeQuery({ kind: 'today' }, today)).toEqual({ from: today.from });
    const yesterday = resolveStatsRange({ kind: 'yesterday' }, now);
    expect(yesterday.to).toBe(today.from - 1);
    expect(new Date(yesterday.from * 1000).getDate()).toBe(10);

    const history = [
      { created: 1, source: 'history' },
      { created: 3, source: 'history' },
    ];
    const live = [
      { created: 2, source: 'live' },
      { created: 3, source: 'live' },
    ];

    expect(mergeStatsByCreated(history, live)).toEqual([
      { created: 1, source: 'history' },
      { created: 2, source: 'live' },
      { created: 3, source: 'live' },
    ]);
  });
});

describe('chart history ordering and gaps', () => {
  it('orders delayed samples, removes invalid timestamps and prefers live duplicates', () => {
    const history = [
      { created: 30, value: 3 },
      { created: 10, value: 1 },
      { created: 20, value: 2 },
    ];
    const live = [
      { created: 25, value: 4 },
      { created: 20, value: 5 },
      { created: Number.NaN, value: 9 },
    ];
    expect(mergeStatsByCreated(history, live)).toEqual([
      { created: 10, value: 1 },
      { created: 20, value: 5 },
      { created: 25, value: 4 },
      { created: 30, value: 3 },
    ]);
    expect(history[0].created).toBe(30);
  });

  it('breaks missing telemetry without replacing real samples or bridging long gaps', () => {
    const samples = [
      { created: 100, value: 1 },
      { created: 110, value: 2 },
      { created: 1000, value: 3 },
    ];
    expect(statsChartData(samples, { from: 0, to: 3600 })).toEqual([
      samples[0],
      samples[1],
      { created: 140 },
      samples[2],
    ]);
    // Wider ranges expect coarser samples, so normal aggregation is not treated as a gap.
    expect(statsChartData(samples, { from: 0, to: 7 * 86400 })).toEqual(samples);
  });
});
