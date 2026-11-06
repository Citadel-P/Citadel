import type { PlatformStatView } from '@/api/generated/api.types';
import { appendPlatformIo, mergePlatformIoSample } from './platform-io';

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

it('sums independently sampled Swarm nodes, including node-only buckets and counter resets', () => {
  let current = sample(3001, [['same', 1000, 100]]);
  let state = appendPlatformIo(undefined, current, 10, 3600);
  const update = (created: number, id: string, bytes: number, disk: number | null, node?: string) => {
    current = mergePlatformIoSample(current, sample(created, [[id, bytes, disk]]), node);
    state = appendPlatformIo(state.series, current, 10, 3600);
  };
  update(3001, 'same', 100, null, 'a');
  update(3001, 'same', 10000, 1000, 'b');
  update(3003, 'same', 1020, 110);
  update(3003, 'same', 10, 10, 'a');
  update(3003, 'same', 10040, 1020, 'b');
  update(3011, 'same', 30, 30, 'a');
  update(3011, 'replacement', 9000, 900, 'b');
  update(3013, 'same', 50, 50, 'a');
  update(3013, 'replacement', 9020, 920, 'b');
  expect(current.created).toBe(3003); // Node updates never pretend host gauges are fresh.
  expect(state.buckets).toMatchObject([
    { rxBytesDelta: 70, rxBytesPerSecond: 35, blockReadBytesDelta: 30, blockReadBytesPerSecond: 15 },
    { rxBytesDelta: 60, rxBytesPerSecond: 14, blockReadBytesDelta: 60, blockReadBytesPerSecond: 14 },
  ]);
  const before = state.buckets;
  update(3013, 'replacement', 9020, 920, 'b');
  update(3001, 'same', 999999, 999999, 'a'); // Delayed observations cannot rewind a node.
  expect(state.buckets).toEqual(before);
  current = mergePlatformIoSample(current, sample(3015, []), 'a');
  state = appendPlatformIo(state.series, current, 10, 3600);
  expect(state.buckets).toEqual(before); // Deletion retains measured traffic.
});

it('retains observations from simultaneous node updates before the next render', () => {
  let current = sample(100, [['host', 10, null]]);
  current = mergePlatformIoSample(current, sample(101, [['task', 100, null]]), 'a');
  current = mergePlatformIoSample(current, sample(102, [['task', 200, null]]), 'b');
  current = mergePlatformIoSample(current, sample(103, [['host', 20, null]]));
  const state = appendPlatformIo(undefined, current, 10, 3600);
  expect(state.series.size).toBe(3);
  expect(current.containerIo?.map((counter) => counter.created)).toEqual([101, 102, 103]);
});

it('rejects delayed host readings even after its last container disappears', () => {
  const empty = mergePlatformIoSample(sample(190, [['host', 100, null]]), sample(200, []));
  const delayed = mergePlatformIoSample(empty, { ...sample(195, [['host', 90, null]]), cpuUsage: 99 });
  expect(delayed).toBe(empty);
});

it('remembers an empty node capture and does not resurrect its delayed counters', () => {
  let current = mergePlatformIoSample(sample(100, []), sample(180, [['task', 100, null]]), 'a');
  current = mergePlatformIoSample(current, sample(200, []), 'a');
  const delayed = mergePlatformIoSample(current, sample(190, [['task', 150, null]]), 'a');
  expect(delayed).toBe(current);
  expect(delayed.containerIo).toEqual([]);
  // An independently sampled node may legitimately have an older timestamp.
  expect(mergePlatformIoSample(current, sample(190, [['task', 50, null]]), 'b').containerIo).toHaveLength(1);
});

it('expires departed-container series when only empty node captures advance time', () => {
  let current = mergePlatformIoSample(sample(100, []), sample(110, [['task', 100, null]]), 'a');
  let state = appendPlatformIo(undefined, current, 10, 60);
  current = mergePlatformIoSample(current, sample(120, [['task', 200, null]]), 'a');
  state = appendPlatformIo(state.series, current, 10, 60);
  expect(state.series.size).toBe(1);
  current = mergePlatformIoSample(current, sample(220, []), 'a');
  state = appendPlatformIo(state.series, current, 10, 60);
  expect(state.series.size).toBe(0);
  expect(current.created).toBe(100); // Empty node captures never refresh the host gauges.
});
