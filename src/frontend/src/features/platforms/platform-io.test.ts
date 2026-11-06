import type { PlatformStatView } from '@/api/generated/api.types';
import { appendPlatformIo } from './platform-io';

const sample = (created: number, counters: [string, number, number | null][]): PlatformStatView => ({
  created,
  cpuUsage: 1,
  memoryUsage: 2,
  rxBytes: 0,
  txBytes: 0,
  diskUsage: null,
  diskUsedBytes: null,
  diskTotalBytes: null,
  containerIo: counters.map(([containerId, bytes, disk]) => ({
    containerId,
    rxBytes: bytes,
    txBytes: bytes,
    blockReadBytes: disk,
    blockWriteBytes: disk,
  })),
});

it('computes rates per physical container before summing and retains departed containers’ traffic', () => {
  let state = appendPlatformIo(
    undefined,
    sample(3001, [
      ['a', 1000, 100],
      ['b', 100, null],
    ]),
    10,
    3600,
  );
  state = appendPlatformIo(
    state.series,
    sample(3003, [
      ['a', 20, 10],
      ['b', 2000, null],
    ]),
    10,
    3600,
  );
  state = appendPlatformIo(
    state.series,
    sample(3009, [
      ['b', 2060, 500],
      ['c', 9000, 900],
    ]),
    10,
    3600,
  );
  state = appendPlatformIo(
    state.series,
    sample(3011, [
      ['b', 2080, 520],
      ['c', 9040, 940],
    ]),
    10,
    3600,
  );
  expect(state.buckets).toMatchObject([
    { rxBytesDelta: 1980, rxBytesPerSecond: 255, blockReadBytesDelta: 10, blockReadBytesPerSecond: 5 },
    { rxBytesDelta: 60, rxBytesPerSecond: 30, blockReadBytesDelta: 60, blockReadBytesPerSecond: 30 },
  ]);
  const repeated = appendPlatformIo(
    state.series,
    sample(3011, [
      ['b', 2080, 520],
      ['c', 9040, 940],
    ]),
    10,
    3600,
  );
  expect(repeated.buckets).toEqual(state.buckets);
  const ended = appendPlatformIo(state.series, sample(3021, []), 10, 3600);
  expect(ended.buckets).toEqual(state.buckets);
});

it('does not derive throughput from aggregate counters without physical baselines', () => {
  const old = { ...sample(100, []), containerIo: undefined, rxBytes: 1000000 };
  expect(appendPlatformIo(undefined, old, 10, 3600).buckets).toEqual([]);
  const first = appendPlatformIo(undefined, sample(110, [['new', 900000, null]]), 10, 3600);
  expect(first.buckets[0]).toMatchObject({ rxBytesDelta: null, rxBytesPerSecond: null, blockReadBytesDelta: null });
});

it('bounds retained departed-container series and starts a new baseline after eviction', () => {
  const first = appendPlatformIo(undefined, sample(100, [['old', 1000, null]]), 10, 60);
  const evicted = appendPlatformIo(first.series, sample(180, []), 10, 60);
  expect(evicted.series.size).toBe(0);
  const restarted = appendPlatformIo(evicted.series, sample(190, [['old', 2000, null]]), 10, 60);
  expect(restarted.buckets[0].rxBytesDelta).toBeNull();
});
