import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { ContainerStatView, StackServiceHistory } from '@/api/generated/api.types';
import { appendLiveStatBucket, latestLiveStat, mergeStatsBuckets, sumStatBuckets } from '@/lib/stats-buckets';

const empty: StackServiceHistory[] = [];
type Patch = { id: string; containerStat?: ContainerStatView | null };

// Keep persisted service identity across replacements. A partial set of replicas
// cannot supply a complete live bucket for a service.
export function useStackSeries(id: string | undefined, history = empty, seconds: number, windowSeconds: number) {
  const memberKey = [...new Set(history.flatMap((s) => s.containerIds))].sort().join(',');
  const membership = useMemo(() => new Set(memberKey.split(',').filter(Boolean)), [memberKey]);
  const [buffer, setBuffer] = useState(() => ({
    id,
    membership,
    seconds,
    byMember: new Map<string, ContainerStatView[]>(),
  }));
  if (buffer.id !== id || buffer.membership !== membership || buffer.seconds !== seconds) {
    const retained =
      id === buffer.id && seconds === buffer.seconds
        ? new Map([...buffer.byMember].filter(([key]) => membership.has(key)))
        : new Map<string, ContainerStatView[]>();
    setBuffer({ id, membership, seconds, byMember: retained });
  }
  const { byMember } = buffer;
  const options = useRef({ membership, seconds, windowSeconds });
  useLayoutEffect(() => {
    options.current = { membership, seconds, windowSeconds };
  }, [membership, seconds, windowSeconds]);
  const receive = useCallback(
    (patches: Patch[], stack: string) => {
      if (stack !== id) return;
      const { membership, seconds, windowSeconds } = options.current;
      setBuffer((buffer) => {
        if (buffer.id !== id || buffer.membership !== membership) return buffer;
        const previous = buffer.byMember;
        let next = previous;
        for (const patch of patches) {
          const key = patch.id;
          const stat = patch.containerStat;
          if (!key || !membership.has(key) || !stat || !Number.isFinite(stat.created) || stat.created <= 0) continue;
          const old = next.get(key) ?? [];
          if (stat.created <= (latestLiveStat(old)?.created ?? 0)) continue;
          if (next === previous) next = new Map(previous);
          next.set(key, appendLiveStatBucket(old, stat, seconds, windowSeconds));
        }
        return next === previous ? buffer : { ...buffer, byMember: next };
      });
    },
    [id],
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
        const members = service.containerIds.map((key) => byMember.get(key) ?? []);
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
