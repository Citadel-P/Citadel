import { useState, useMemo } from 'react';
import type { PlatformView, PlatformStatView } from '@/api/generated/api.types';
import { appendPlatformIo, platformIoFields, type PlatformIoSeries } from './platform-io';
import { appendLiveStatBucket, mergeStatsBuckets } from '@/lib/stats-buckets';
export type PlatformStatsDatum = Pick<PlatformStatView, (typeof platformIoFields)[number]> & {
  created: number;
  cpuUsage: number | null;
  memoryUsage: number | null;
  rxBytes: number | null;
  txBytes: number | null;
  diskUsedBytes: number | null;
  diskTotalBytes: number | null;
  diskUsage: number | null;
};

export const usePlatformLiveStats = (
  platform: PlatformView | undefined,
  seconds: number,
  windowSeconds: number,
): PlatformStatsDatum[] => {
  const latestStat = platform?.stats?.at(0);
  const nextState = (previous?: { gauges: PlatformStatsDatum[]; io: PlatformIoSeries }) => {
    const normalized =
      latestStat && (latestStat.cpuUsage != null || latestStat.memoryUsage != null)
        ? normalizePlatformStats([latestStat])[0]
        : undefined;
    const gauges = normalized
      ? appendLiveStatBucket(previous?.gauges ?? [], normalized, seconds, windowSeconds)
      : (previous?.gauges ?? []);
    const { series: io, buckets } = appendPlatformIo(previous?.io, latestStat, seconds, windowSeconds);
    const byBucket = new Map(buckets.map((point) => [Math.floor(point.created / seconds), point]));
    const combined = new Map(gauges.map((point) => [Math.floor(point.created / seconds), point]));
    for (const point of buckets) {
      const bucket = Math.floor(point.created / seconds);
      if (!combined.has(bucket))
        combined.set(bucket, {
          created: point.created,
          cpuUsage: null,
          memoryUsage: null,
          rxBytes: null,
          txBytes: null,
          diskUsage: null,
          diskUsedBytes: null,
          diskTotalBytes: null,
        });
    }
    return {
      platformId: platform?.id,
      latestStat,
      seconds,
      gauges,
      io,
      stats: [...combined.values()]
        .sort((a, b) => a.created - b.created)
        .map((point) => {
          const measured = byBucket.get(Math.floor(point.created / seconds));
          return { ...point, ...Object.fromEntries(platformIoFields.map((key) => [key, measured?.[key] ?? null])) };
        }),
    };
  };
  const [live, setLive] = useState(() => nextState());
  if (live.platformId !== platform?.id || live.latestStat !== latestStat || live.seconds !== seconds) {
    const next = nextState(live.platformId === platform?.id && live.seconds === seconds ? live : undefined);
    setLive(next);
    return next.stats;
  }
  return live.stats;
};

export const useCombinedStats = (baseStats: PlatformStatView[], liveStats: PlatformStatsDatum[], seconds: number) => {
  const normalizedBaseStats = useMemo(() => normalizePlatformStats(baseStats), [baseStats]);
  return useMemo(
    () => mergeStatsBuckets(normalizedBaseStats, liveStats, seconds),
    [liveStats, normalizedBaseStats, seconds],
  );
};

export const normalizePlatformStats = (stats?: PlatformStatView[] | null): PlatformStatsDatum[] =>
  (stats ?? [])
    .map((stat) => {
      const diskUsage = stat.diskUsage == null ? null : Number(stat.diskUsage);
      const diskUsedBytes = stat.diskUsedBytes == null ? null : Number(stat.diskUsedBytes);
      const diskTotalBytes = stat.diskTotalBytes == null ? null : Number(stat.diskTotalBytes);
      const hasCompleteDiskSample =
        diskUsage != null &&
        Number.isFinite(diskUsage) &&
        diskUsage >= 0 &&
        diskUsage <= 100 &&
        diskUsedBytes != null &&
        Number.isFinite(diskUsedBytes) &&
        diskUsedBytes >= 0 &&
        diskTotalBytes != null &&
        Number.isFinite(diskTotalBytes) &&
        diskTotalBytes > 0 &&
        diskUsedBytes <= diskTotalBytes;

      return {
        ...Object.fromEntries(platformIoFields.map((key) => [key, stat[key] ?? null])),
        // Explicit unknown deltas prevent deriving rates from aggregate lifetime counters.
        rxBytesDelta: stat.rxBytesDelta ?? null,
        txBytesDelta: stat.txBytesDelta ?? null,
        rxBytesPerSecond: stat.rxBytesPerSecond ?? null,
        txBytesPerSecond: stat.txBytesPerSecond ?? null,
        created: Number(stat.created ?? 0),
        cpuUsage: stat.cpuUsage == null ? null : Number(stat.cpuUsage),
        memoryUsage: stat.memoryUsage == null ? null : Number(stat.memoryUsage),
        rxBytes: stat.rxBytes == null ? null : Number(stat.rxBytes),
        txBytes: stat.txBytes == null ? null : Number(stat.txBytes),
        diskUsedBytes: hasCompleteDiskSample ? diskUsedBytes : null,
        diskTotalBytes: hasCompleteDiskSample ? diskTotalBytes : null,
        diskUsage: hasCompleteDiskSample ? diskUsage : null,
      };
    })
    .filter((stat) => stat.created > 0)
    .sort((a, b) => a.created - b.created);
