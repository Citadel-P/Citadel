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
