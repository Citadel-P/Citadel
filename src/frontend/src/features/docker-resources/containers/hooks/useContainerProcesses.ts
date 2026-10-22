import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { ContainerProcessesView } from '@/api/generated/api.types';
import type { RealtimeConnection } from '@/lib/realtime-connection';

type Update = { resourceId: string; data: ContainerProcessesView | null };
const PROCESS_STALE_TIME = 30_000;

// Mount only while the section is open, keyed by the persisted container ID.
export function useContainerProcesses(resourceId: string) {
  const queryClient = useQueryClient();
  const query = useRead(
    'getContainerProcesses',
    { id: resourceId },
    {
      staleTime: PROCESS_STALE_TIME,
      gcTime: 5 * 60_000,
      retry: false,
      meta: { suppressErrorToast: true },
    },
  );
  const [live, setLive] = useState<Update>();
  const revision = useRef(0);
  const joined = useRef(false);
  const { refetch } = query;
  const refresh = useCallback(async () => {
    const before = revision.current;
    const result = await refetch({ cancelRefetch: false });
    if (before === revision.current) setLive(result.isSuccess ? undefined : { resourceId, data: null });
  }, [refetch, resourceId]);
  const receive = useCallback(
    (update: Update) => {
      if (update.resourceId !== resourceId) return;
      revision.current += 1;
      setLive(update);
      const filter = { queryKey: ['getContainerProcesses', { id: resourceId }], exact: true };
      if (update.data) {
        // A late HTTP response must not replace this newer snapshot in the cache.
        void queryClient.cancelQueries(filter);
        queryClient.setQueryData(filter.queryKey, { data: update.data });
      } else {
        // Keep the error local, but don't reuse a fresh-looking snapshot on reopen.
        void queryClient.invalidateQueries({ ...filter, refetchType: 'none' });
      }
    },
    [queryClient, resourceId],
  );
  const reconnect = useCallback(
    async (notification: { resourceId: string }) => {
      if (notification.resourceId !== resourceId) return;
      const filter = { queryKey: ['getContainerProcesses', { id: resourceId }], exact: true, type: 'active' as const };
      if (!joined.current) {
        joined.current = true;
        const cached = queryClient.getQueryState(filter.queryKey);
        // Joining after expansion should reuse the mount's GET or fresh cache.
        // Later joins on this mount are reconnects and must resynchronize.
        if (
          queryClient.isFetching(filter) ||
          (cached?.status === 'success' &&
            !cached.isInvalidated &&
            Date.now() - cached.dataUpdatedAt < PROCESS_STALE_TIME)
        ) {
          return;
        }
      }
      const before = revision.current;
      if (queryClient.isFetching(filter)) await queryClient.refetchQueries(filter, { cancelRefetch: false });
      await queryClient.invalidateQueries(filter, { cancelRefetch: false });
      if (before === revision.current) {
        setLive(
          queryClient.getQueryState(filter.queryKey)?.status === 'success' ? undefined : { resourceId, data: null },
        );
      }
    },
    [queryClient, resourceId],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('ContainerProcessesUpdated', receive);
      connection.on('ContainerProcessesRefresh', reconnect);
    },
    [receive, reconnect],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('ContainerProcessesUpdated', receive);
      connection.off('ContainerProcessesRefresh', reconnect);
    },
    [receive, reconnect],
  );
  const { isConnected } = useRealtimeGroup({
    groupName: `container-processes:${resourceId}`,
    setupEventListeners,
    removeEventListeners,
  });
  return {
    data: live ? live.data : query.data?.data,
    isError: live ? live.data === null : query.isError,
    isLoading: !live && query.isLoading,
    isFetching: query.isFetching,
    isConnected,
    refresh,
  };
}
