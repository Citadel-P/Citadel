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

/** Retain departed containers' measured traffic, but never use another container as a baseline. */
export function appendPlatformIo(
  current: PlatformIoSeries | undefined,
  sample: PlatformStatView | undefined,
  seconds: number,
  windowSeconds: number,
) {
  const series: PlatformIoSeries = new Map(current);
  if (sample && Number(sample.created) > 0) {
    for (const [id, points] of series) {
      const retained = points.filter((point) => point.created >= sample.created - windowSeconds - seconds);
      if (retained.length) series.set(id, retained);
      else series.delete(id);
    }
    for (const counters of sample.containerIo ?? []) {
      series.set(
        counters.containerId,
        appendLiveStatBucket(
          series.get(counters.containerId) ?? [],
          { ...counters, created: sample.created },
          seconds,
          windowSeconds,
        ),
      );
    }
  }
  return { series, buckets: sumStatBuckets([...series.values()], seconds) };
}
