import { useCallback } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useQueryClient } from '@tanstack/react-query';
import { ResourceControlState, StackView } from '@/api/generated/api.types';
import { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useStackGroup = (stackId: string) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('getStack', { stackId });
  const queryClient = useQueryClient();
  const handleStackInfoUpdated = useCallback(
    (stack: StackView, action: string) => {
      if (stack.id !== stackId) return;
      const driftQueryKey = ['getStackDrift', { stackId }];
      if (action === 'delete') {
        void queryClient.cancelQueries({ queryKey: driftQueryKey });
        return;
      }
      const activityInfo = stack.latestActivityView?.info;
      const info = Array.isArray(activityInfo) ? { ...activityInfo[1], $type: activityInfo[0] } : activityInfo;

      queryClient.setQueryData<ResourceResponse<'getStack'>>(['getStack', { stackId }], (current) =>
        current?.data
          ? {
              ...current,
              data: {
                ...current.data,
                name: stack.name,
                status: stack.status,
                description: stack.description,
                controlState: stack.controlState,
                driftPolicy: stack.driftPolicy,
                stackUpdateState: stack.stackUpdateState,
                platformStatus: stack.platformStatus,
                platformName: stack.platformName,
                latestActivityView: stack.latestActivityView ? { ...stack.latestActivityView, info } : null,
              },
            }
          : current,
      );

      if (stack.controlState === ResourceControlState.Processing) {
        void queryClient.cancelQueries({ queryKey: driftQueryKey });
      } else {
        void queryClient.invalidateQueries({ queryKey: driftQueryKey });
      }
    },
    [queryClient, stackId],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('StackInfoUpdated', handleStackInfoUpdated);
    },
    [handleStackInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('StackInfoUpdated', handleStackInfoUpdated);
    },
    [handleStackInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'stack:' + stackId,
    skip: !stackId,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, stack: data?.data, isLoading };
};
