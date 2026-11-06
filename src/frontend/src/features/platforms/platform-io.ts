import type { ContainerStatView, PlatformStatView } from '@/api/generated/api.types';
import { sumStatBuckets, appendLiveStatBucket } from '@/lib/stats-buckets';

export const platformIoFields = [
  'rxBytesDelta',
  'txBytesDelta',
  'rxBytesPerSecond',
  'txBytesPerSecond',
  'blockReadBytes',
  'blockWriteBytes',
  'blockReadBytesDelta',
  'blockWriteBytesDelta',
  'blockReadBytesPerSecond',
  'blockWriteBytesPerSecond',
] as const;
type Point = Pick<
  ContainerStatView,
  'created' | 'containerId' | 'rxBytes' | 'txBytes' | (typeof platformIoFields)[number]
>;
export type PlatformIoSeries = Map<string, Point[]>;

// Like stats-buckets' accumulators, these capture times stay in client memory.
// An empty capture still advances its source, without making host gauges fresher.
const captureTimes = Symbol('platformIoCaptureTimes');
const retentionSeconds = 7 * 24 * 3600;
type ObservedPlatformStat = PlatformStatView & { [captureTimes]?: ReadonlyMap<string, number> };

function sourceCaptureTimes(sample?: ObservedPlatformStat): Map<string, number> {
  if (!sample) return new Map();
  if (sample[captureTimes]) return new Map(sample[captureTimes]);
  const times = new Map([['', sample.created]]);
  for (const counter of sample.containerIo ?? []) {
    const source = counter.nodeId ?? '';
    times.set(source, Math.max(times.get(source) ?? 0, counter.created ?? sample.created));
  }
  return times;
}

/** Retain departed containers' measured traffic, but never use another container as a baseline. */
export function appendPlatformIo(
  current: PlatformIoSeries | undefined,
  sample: PlatformStatView | undefined,
  seconds: number,
  windowSeconds: number,
) {
  const series: PlatformIoSeries = new Map(current);
  if (sample && Number(sample.created) > 0) {
    let newest = sample.created;
    for (const time of sourceCaptureTimes(sample).values()) newest = Math.max(newest, time);
    for (const points of series.values()) newest = Math.max(newest, points.at(-1)?.created ?? 0);
    for (const [id, points] of series) {
      const retained = points.filter((point) => point.created >= newest - windowSeconds - seconds);
      if (retained.length) series.set(id, retained);
      else series.delete(id);
    }
    for (const counters of sample.containerIo ?? []) {
      const created = counters.created ?? sample.created;
      if (created < newest - windowSeconds - seconds) continue;
      const id = JSON.stringify([counters.nodeId ?? '', counters.containerId]);
      series.set(id, appendLiveStatBucket(series.get(id) ?? [], { ...counters, created }, seconds, windowSeconds));
    }
  }
  return { series, buckets: sumStatBuckets([...series.values()], seconds) };
}

/** Merge raw observations without replacing the connected host's gauges on node updates. */
export function mergePlatformIoSample(
  previous: PlatformStatView | undefined,
  incoming: PlatformStatView,
  nodeId?: string,
): PlatformStatView {
  const source = nodeId ?? '';
  const times = sourceCaptureTimes(previous);
  let newest = incoming.created;
  for (const time of times.values()) newest = Math.max(newest, time);
  const cutoff = newest - retentionSeconds;
  if (previous && (incoming.created < (times.get(source) ?? 0) || incoming.created < cutoff)) return previous;
  times.set(source, incoming.created);
  for (const [id, time] of times) if (id !== '' && time < cutoff) times.delete(id);
  const retained = (previous?.containerIo ?? []).map((c) => ({ ...c, created: c.created ?? previous!.created }));
  const counters = (incoming.containerIo ?? []).map((c) => ({ ...c, nodeId, created: incoming.created }));
  const merged: ObservedPlatformStat = {
    ...(nodeId && previous ? previous : incoming),
    containerIo: [...retained.filter((c) => (c.nodeId ?? '') !== source && c.created >= cutoff), ...counters],
    [captureTimes]: times,
  };
  return merged;
}
