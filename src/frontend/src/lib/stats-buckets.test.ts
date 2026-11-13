import {
  appendLiveStatBucket,
  averageStatBuckets,
  latestLiveStat,
  mergeStatsBuckets,
  sumStatBuckets,
} from './stats-buckets';
import { statsBucketSeconds } from './stats-range';

describe('historical and live statistics buckets', () => {
  it.each([3600, 21600, 86400, 172800, 604800])('uses the server resolution for a %s-second range', (duration) => {
    const seconds = statsBucketSeconds({ from: 0, to: duration }, duration);
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

it('sums raw network increases before averaging and preserves resets within a bucket', () => {
  const samples = [
    { created: 301, rxBytes: 1000, txBytes: 2000 },
    { created: 311, rxBytes: 1100, txBytes: 2040 },
    { created: 321, rxBytes: 5, txBytes: 2060 },
    { created: 331, rxBytes: 25, txBytes: 3 },
  ];
  const live = samples.reduce(
    (buckets, sample) => appendLiveStatBucket(buckets, sample, 300, 3600),
    [] as typeof samples,
  );
  expect(live).toHaveLength(1);
  expect(live[0]).toMatchObject({ rxBytesDelta: 125, txBytesDelta: 63 });
  expect(latestLiveStat(live)).toEqual(samples.at(-1));
  const corrected = appendLiveStatBucket(live, { created: 331, rxBytes: 30, txBytes: 4 }, 300, 3600);
  expect(corrected[0]).toMatchObject({ rxBytesDelta: 130, txBytesDelta: 64 });
  const next = appendLiveStatBucket(corrected, { created: 601, rxBytes: 40, txBytes: 14 }, 300, 3600);
  expect(next[1]).toMatchObject({ rxBytesDelta: 10, txBytesDelta: 10 });
  const historical = [{ created: 301, rxBytes: 9, txBytes: 10, rxBytesDelta: 140, txBytesDelta: 70 }];
  expect(
    mergeStatsBuckets<(typeof samples)[number] & { rxBytesDelta?: number; txBytesDelta?: number }>(
      historical,
      next,
      300,
    ),
  ).toMatchObject([
    { rxBytesDelta: 140, txBytesDelta: 70 },
    { rxBytesDelta: 10, txBytesDelta: 10 },
  ]);
});

it('does not count a replacement container lifetime counter as live traffic', () => {
  const first = appendLiveStatBucket([], { containerId: 'old', created: 10, rxBytes: 1000 }, 10, 900);
  const replacement = appendLiveStatBucket(first, { containerId: 'new', created: 20, rxBytes: 9000 }, 10, 900);
  expect(replacement[1]).toMatchObject({ rxBytesDelta: null });
  expect(
    appendLiveStatBucket(replacement, { containerId: 'new', created: 30, rxBytes: 9030 }, 10, 900)[2],
  ).toMatchObject({ rxBytesDelta: 30 });
});

it('derives network and disk throughput from raw intervals before averaging, including counter resets and missing baselines', () => {
  const samples = [
    { created: 301, rxBytes: 1000, blockReadBytes: null as number | null, blockWriteBytes: 1000 },
    { created: 311, rxBytes: 1100, blockReadBytes: 10000, blockWriteBytes: 1200 },
    { created: 331, rxBytes: 10, blockReadBytes: 10200, blockWriteBytes: 20 },
  ];
  const live = samples.reduce(
    (buckets, sample) => appendLiveStatBucket(buckets, sample, 300, 3600),
    [] as typeof samples,
  );
  expect(live[0]).toMatchObject({
    rxBytesDelta: 110,
    rxBytesPerSecond: 110 / 30,
    blockReadBytesDelta: 200,
    blockReadBytesPerSecond: 10,
    blockWriteBytesDelta: 220,
    blockWriteBytesPerSecond: 220 / 30,
  });
  const corrected = appendLiveStatBucket(live, { ...samples[2], rxBytes: 30 }, 300, 3600);
  expect(corrected[0]).toMatchObject({ rxBytesDelta: 130, rxBytesPerSecond: 130 / 30 });
  const replacement = appendLiveStatBucket([], { containerId: 'old', created: 1, blockReadBytes: 10 }, 10, 900);
  expect(
    appendLiveStatBucket(replacement, { containerId: 'new', created: 11, blockReadBytes: 1000 }, 10, 900)[1],
  ).toMatchObject({ blockReadBytesDelta: null, blockReadBytesPerSecond: null });
});

it('weights throughput by elapsed time rather than the number of observations', () => {
  const samples = [
    { created: 301, rxBytes: 1000, blockReadBytes: 1000 },
    { created: 302, rxBytes: 2000, blockReadBytes: 2000 },
    { created: 322, rxBytes: 2000, blockReadBytes: 2000 },
  ];
  const live = samples.reduce(
    (buckets, sample) => appendLiveStatBucket(buckets, sample, 300, 3600),
    [] as typeof samples,
  );
  expect(live[0]).toMatchObject({
    rxBytesDelta: 1000,
    rxBytesPerSecond: 1000 / 21,
    blockReadBytesPerSecond: 1000 / 21,
  });
  const corrected = appendLiveStatBucket(live, { ...samples[2], rxBytes: 2100 }, 300, 3600);
  expect(corrected[0]).toMatchObject({ rxBytesDelta: 1100, rxBytesPerSecond: 1100 / 21 });
});

it('keeps active-cache breakdown unknown for mixed coverage and corrects duplicate samples', () => {
  let live = appendLiveStatBucket(
    [],
    { created: 301, memoryActive: 100, memoryActiveCache: null as number | null },
    60,
    3600,
  );
  live = appendLiveStatBucket(live, { created: 311, memoryActive: 100, memoryActiveCache: 20 }, 60, 3600);
  expect(live[0].memoryActiveCache).toBeNull();
  expect(live[0].memoryActive).toBe(100);
  let known = appendLiveStatBucket([], { created: 301, memoryActiveCache: 20 }, 60, 3600);
  known = appendLiveStatBucket(known, { created: 311, memoryActiveCache: 40 }, 60, 3600);
  known = appendLiveStatBucket(known, { created: 311, memoryActiveCache: 60 }, 60, 3600);
  expect(known[0].memoryActiveCache).toBe(40);
});

it('does not report a partial active-cache sum as complete replica coverage', () => {
  const known = { created: 301, memoryActive: 100, memoryActiveCache: 20 as number | null };
  expect(sumStatBuckets([[known], [{ ...known, memoryActiveCache: null }]], 60)[0].memoryActiveCache).toBeNull();
  expect(sumStatBuckets([[known], [known]], 60)[0].memoryActiveCache).toBe(40);
});
