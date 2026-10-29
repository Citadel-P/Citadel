import { appendLiveStatBucket, averageStatBuckets, latestLiveStat, mergeStatsBuckets } from './stats-buckets';
import { statsBucketSeconds } from './stats-range';

describe('historical and live statistics buckets', () => {
  it.each([3600, 21600, 86400, 172800, 604800])('uses the server resolution for a %s-second range', (duration) => {
    const seconds = statsBucketSeconds({ from: 0, to: duration });
    const samples = Array.from({ length: (2 * seconds) / 10 }, (_, index) => ({
      created: index * 10 + 1,
      cpuUsage: index === 0 ? 28 : 1,
    }));
    const live = samples.reduce(
      (current, sample) => appendLiveStatBucket(current, sample, seconds, duration),
      [] as typeof samples,
    );
    // Equivalent to GROUP BY created / bucket_seconds, AVG(cpuusage).
    expect(live.map(({ cpuUsage }) => cpuUsage)).toEqual([1 + 27 / (seconds / 10), 1]);
    expect(averageStatBuckets(live, seconds).map(({ cpuUsage }) => cpuUsage)).toEqual(
      live.map(({ cpuUsage }) => cpuUsage),
    );
    expect(latestLiveStat(live)).toEqual(samples.at(-1));
  });

  it('keeps exact bucket averages after days of live updates without sampling away spikes', () => {
    let live: { created: number; cpuUsage: number }[] = [];
    for (let created = 1; created < 3 * 86400; created += 10) {
      live = appendLiveStatBucket(live, { created, cpuUsage: created % 300 === 1 ? 28 : 1 }, 300, 172800);
    }
    expect(live.length).toBeLessThanOrEqual(578);
    expect(live.length).toBeGreaterThanOrEqual(576);
    expect(live.every(({ cpuUsage }) => cpuUsage === 1.9)).toBe(true);
    expect(latestLiveStat(live)?.cpuUsage).toBe(1);
  });

  it('replaces duplicate observations without counting them twice and ignores stale replay', () => {
    const first = appendLiveStatBucket([], { created: 301, cpuUsage: 2 }, 300, 172800);
    const second = appendLiveStatBucket(first, { created: 311, cpuUsage: 10 }, 300, 172800);
    const corrected = appendLiveStatBucket(second, { created: 311, cpuUsage: 4 }, 300, 172800);
    expect(corrected[0].cpuUsage).toBe(3);
    expect(first[0].cpuUsage).toBe(2);
    expect(second[0].cpuUsage).toBe(6);
    expect(appendLiveStatBucket(corrected, { created: 301, cpuUsage: 90 }, 300, 172800)).toEqual(corrected);
  });

  it('does not mix historical averages with partial observations from the same bucket', () => {
    const history = [{ created: 301, cpuUsage: 2 }];
    const live = [
      { created: 591, cpuUsage: 28 },
      { created: 601, cpuUsage: 28 },
      { created: 611, cpuUsage: 2 },
    ];
    expect(mergeStatsBuckets(history, live, 300).map(({ created, cpuUsage }) => ({ created, cpuUsage }))).toEqual([
      { created: 301, cpuUsage: 2 },
      { created: 601, cpuUsage: 15 },
    ]);
    expect(mergeStatsBuckets([...history, { created: 601, cpuUsage: 3 }], live, 300).map((s) => s.cpuUsage)).toEqual([
      2, 3,
    ]);
  });

  it('does not treat null disk measurements as zero or reuse a coarse bucket at a finer resolution', () => {
    const live = [
      { created: 301, diskUsage: null },
      { created: 311, diskUsage: 20 },
    ].reduce(
      (current, sample) => appendLiveStatBucket(current, sample, 300, 172800),
      [] as { created: number; diskUsage: number | null }[],
    );
    expect(live[0].diskUsage).toBe(20);
    expect(mergeStatsBuckets([], live, 10)).toEqual([]);
    expect(appendLiveStatBucket(live, { created: 321, diskUsage: 40 }, 10, 3600).map((s) => s.diskUsage)).toEqual([40]);
  });
});
