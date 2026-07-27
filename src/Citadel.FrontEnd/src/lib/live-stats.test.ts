import { appendBoundedLiveStat, mergeStatsByCreated } from './live-stats';

describe('live statistics retention', () => {
  it('retains only the newest configured number of samples', () => {
    const stats = [1, 2, 3].reduce(
      (current, created) => appendBoundedLiveStat(current, { created }, 2),
      [] as Array<{ created: number }>,
    );

    expect(stats).toEqual([{ created: 2 }, { created: 3 }]);
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
