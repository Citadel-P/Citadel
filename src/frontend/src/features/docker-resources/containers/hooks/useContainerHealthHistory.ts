import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ContainerHealthTransitionView } from '@/api/generated/api.types';
import type { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useContainerHealthHistory(resourceId: string) {
  const queryClient = useQueryClient();
  // The group supplies the initial snapshot and resynchronizes after reconnect.
  // Keep the standard query cache, but never start an HTTP read or polling timer.
  const query = useRead('getContainerHealthHistory', { id: resourceId }, { enabled: false, staleTime: Infinity });
  const receive = useCallback(
    (snapshot: { resourceId: string; entries: ContainerHealthTransitionView[] }) => {
      if (snapshot.resourceId !== resourceId) return;
      const queryKey = ['getContainerHealthHistory', { id: resourceId }] as const;
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<ResourceResponse<'getContainerHealthHistory'>>(
        queryKey,
        (current) =>
          ({
            ...current,
            data: snapshot.entries,
          }) as ResourceResponse<'getContainerHealthHistory'>,
      );
    },
    [queryClient, resourceId],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('ReceiveContainerHealthHistory', receive);
    },
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('ReceiveContainerHealthHistory', receive);
    },
    [receive],
  );
  const { isConnected, isLoading } = useRealtimeGroup({
    groupName: `container-health:${resourceId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !resourceId,
  });
  return { entries: query.data?.data, isConnected, isLoading: !query.data && (isLoading || isConnected) };
}
