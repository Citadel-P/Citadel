import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { JobsSnapshotView } from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { canShowCachedResource } from '@/lib/request-error';

const queryKey = resourceQueryKey('getInstanceJobs');

export function useInstanceJobs() {
  const client = useQueryClient();
  const query = useRead('getInstanceJobs', undefined, {
    retry: false,
    staleTime: Infinity,
    gcTime: 0,
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    meta: { suppressErrorToast: true },
  });
  const [live, setLive] = useState<JobsSnapshotView>();
  const [synchronized, setSynchronized] = useState(false);
  const latest = useRef<JobsSnapshotView | undefined>(undefined);
  const retiredBoot = useRef<string | undefined>(undefined);
  const receive = useCallback(
    (snapshot: JobsSnapshotView) => {
      // An already queued event must not restore data after HTTP confirmed revocation.
      if (!canShowCachedResource(client.getQueryState(queryKey)?.error)) return;
      const previous = latest.current;
      if (snapshot.bootId === retiredBoot.current) return;
      if (previous?.bootId === snapshot.bootId && snapshot.revision < previous.revision) return;
      if (previous && previous.bootId !== snapshot.bootId) retiredBoot.current = previous.bootId;
      latest.current = snapshot;
      setLive(snapshot);
      setSynchronized(true);
      // Abort the initial HTTP read before it can overwrite the authoritative stream.
      void client.cancelQueries({ queryKey, exact: true });
      client.setQueryData(queryKey, { data: snapshot });
    },
    [client],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      setSynchronized(false);
      connection.on('InstanceJobsUpdated', receive);
    },
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('InstanceJobsUpdated', receive);
    },
    [receive],
  );
  const { refetch } = query;
  const verifyAccess = useCallback(() => {
    void refetch();
  }, [refetch]);
  const startStream = useCallback(
    (connection: RealtimeConnection) => {
      // Joining authorizes the group; streaming requires a separate invocation.
      // The shared group hook repeats this after reconnect, with listeners installed first.
      void connection.invoke('StartInstanceJobs').catch(verifyAccess);
    },
    [verifyAccess],
  );
  const realtime = useRealtimeGroup({
    groupName: 'instance-jobs',
    setupEventListeners,
    removeEventListeners,
    onSubscriptionError: verifyAccess,
    onJoinedGroup: startStream,
  });
  const denied = !canShowCachedResource(query.error);
  return {
    data: denied ? undefined : (live ?? query.data?.data),
    denied,
    isLoading: !live && query.isPending,
    isError: query.isError,
    isLive: realtime.isConnected && synchronized,
    isConnecting: realtime.isLoading,
    retry: query.refetch,
  };
}
