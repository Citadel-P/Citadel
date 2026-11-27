import { useCallback, useState } from 'react';
import type { ContainerStatView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { appendLiveStatBucket, latestLiveStat, sumStatBuckets } from '@/lib/stats-buckets';

type Sample = ContainerStatView;
const empty = (group?: string, seconds?: number) => ({
  group,
  seconds,
  expected: undefined as number | undefined,
  members: new Set<string>(),
  recent: new Map<string, Sample[]>(),
  stats: [] as Sample[],
});

export function useWorkloadSeries(group: string | undefined, seconds: number, windowSeconds: number) {
  const [buffer, setBuffer] = useState(() => empty(group, seconds));
  if (buffer.group !== group || buffer.seconds !== seconds) setBuffer(empty(group, seconds));
  const membership = useCallback(
    (message: { group: string; members: string[]; expected: number }) => {
      if (message.group === group)
        setBuffer({ ...empty(group, seconds), expected: message.expected, members: new Set(message.members) });
    },
    [group, seconds],
  );
  const receive = useCallback(
    (message: { group: string; stats: Sample[] }) => {
      if (message.group !== group) return;
      setBuffer((previous) => {
        const recent = new Map(previous.recent);
        const changed = new Set<number>();
        let latest = previous.stats.at(-1)?.created ?? 0;
        for (const sample of message.stats) {
          if (!previous.members.has(sample.containerId) || !Number.isFinite(sample.created)) continue;
          const before = recent.get(sample.containerId) ?? [];
          if (sample.created <= (latestLiveStat(before)?.created ?? 0)) continue;
          // Only the current and preceding buckets are needed per physical container.
          recent.set(sample.containerId, appendLiveStatBucket(before, sample, seconds, seconds));
          latest = Math.max(latest, sample.created);
          changed.add(Math.floor(sample.created / seconds));
        }
        if (!changed.size) return previous;
        const complete = new Map(
          previous.stats
            .filter((s) => s.created >= latest - windowSeconds - seconds)
            .map((s) => [Math.floor(s.created / seconds), s]),
        );
        for (const bucket of changed) {
          const contributions = [...recent.values()].map((samples) =>
            samples.filter((s) => Math.floor(s.created / seconds) === bucket),
          );
          if (
            contributions.length === previous.members.size &&
            contributions.every((samples) => samples.length === 1)
          ) {
            complete.set(bucket, sumStatBuckets(contributions, seconds)[0]);
          }
        }
        return {
          ...previous,
          recent,
          stats: [...complete.values()].sort((a, b) => a.created - b.created),
        };
      });
    },
    [group, seconds, windowSeconds],
  );
  const setupEventListeners = useCallback(
    (c: RealtimeConnection) => {
      c.on('WorkloadStatsMembership', membership);
      c.on('WorkloadStatsUpdated', receive);
    },
    [membership, receive],
  );
  const removeEventListeners = useCallback(
    (c: RealtimeConnection) => {
      c.off('WorkloadStatsMembership', membership);
      c.off('WorkloadStatsUpdated', receive);
    },
    [membership, receive],
  );
  const { isConnected } = useRealtimeGroup({ groupName: group, setupEventListeners, removeEventListeners });
  return isConnected ? buffer : unavailable;
}
const unavailable = empty();
