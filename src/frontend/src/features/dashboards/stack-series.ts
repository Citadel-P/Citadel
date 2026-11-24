import { useCallback, useMemo, useState } from 'react';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { ContainerStatView, StackServiceHistory } from '@/api/generated/api.types';
import { appendLiveStatBucket, latestLiveStat, mergeStatsBuckets, sumStatBuckets } from '@/lib/stats-buckets';
import { normalizeDockerId } from '@/lib/utils';

const empty: StackServiceHistory[] = [];
type Patch = { id: string; containerStat?: ContainerStatView | null };

// Keep persisted service identity across replacements. A partial set of replicas
// cannot supply a complete live bucket for a service.
export function useStackSeries(id: string | undefined, history = empty, seconds: number, windowSeconds: number) {
  const membership = useMemo(() => new Set(history.flatMap((s) => s.containerIds.map(normalizeDockerId))), [history]);
  const [byMember, setByMember] = useState(new Map<string, ContainerStatView[]>());
  const receive = useCallback(
    (patches: Patch[]) => {
      setByMember((previous) => {
        let next = previous;
        for (const patch of patches) {
          const key = normalizeDockerId(patch.id);
          const stat = patch.containerStat;
          if (!key || !membership.has(key) || !stat || !Number.isFinite(stat.created) || stat.created <= 0) continue;
          const old = previous.get(key) ?? [];
          if (stat.created <= (latestLiveStat(old)?.created ?? 0)) continue;
          if (next === previous) next = new Map([...previous].filter(([k]) => membership.has(k)));
          next.set(key, appendLiveStatBucket(old, stat, seconds, windowSeconds));
        }
        return next;
      });
    },
    [membership, seconds, windowSeconds],
  );
  const setupEventListeners = useCallback(
    (c: RealtimeConnection) => c.on('ReceiveStackContainerStats', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (c: RealtimeConnection) => c.off('ReceiveStackContainerStats', receive),
    [receive],
  );
  useRealtimeGroup({
    groupName: id ? 'stack-info:' + id : undefined,
    enabled: !!id,
    setupEventListeners,
    removeEventListeners,
  });
  return useMemo(
    () =>
      history.map((service) => {
        const members = service.containerIds.map((key) => byMember.get(normalizeDockerId(key) ?? '') ?? []);
        if (!members.length || members.some((m) => !m.length))
          return { name: service.serviceName, stats: service.stats };
        const coverage = new Map<number, number>();
        for (const samples of members)
          for (const sample of samples) {
            const bucket = Math.floor(sample.created / seconds);
            coverage.set(bucket, (coverage.get(bucket) ?? 0) + 1);
          }
        const live = sumStatBuckets(members, seconds).filter(
          (s) => coverage.get(Math.floor(s.created / seconds)) === members.length,
        );
        return { name: service.serviceName, stats: mergeStatsBuckets(service.stats, live, seconds) };
      }),
    [history, byMember, seconds],
  );
}
