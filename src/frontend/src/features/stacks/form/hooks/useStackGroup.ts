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
      const previous = queryClient.getQueryData<ResourceResponse<'getStack'>>(['getStack', { stackId }])?.data;
      const activityInfo = stack.latestActivityView?.info;
      const info = Array.isArray(activityInfo) ? { ...activityInfo[1], $type: activityInfo[0] } : activityInfo;
      const filter = { queryKey: ['getStack', { stackId }], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData<{ data: StackView }>(filter.queryKey, (current) => ({
        ...current,
        data: {
          ...current?.data,
          ...stack,
          backupCoverage: stack.backupCoverage ?? current?.data.backupCoverage,
          copyEligibility: stack.copyEligibility ?? current?.data.copyEligibility,
          capabilities: stack.capabilities ?? current?.data.capabilities ?? null,
          latestActivityView: stack.latestActivityView ? { ...stack.latestActivityView, info } : null,
        },
      }));

      if (stack.controlState === ResourceControlState.Processing) {
        void queryClient.cancelQueries({ queryKey: driftQueryKey });
      } else if (
        !previous ||
        previous.controlState !== stack.controlState ||
        previous.status !== stack.status ||
        previous.platformId !== stack.platformId ||
        previous.platformStatus !== stack.platformStatus ||
        previous.currentStackReleaseId !== stack.currentStackReleaseId ||
        JSON.stringify(previous.driftPolicy) !== JSON.stringify(stack.driftPolicy) ||
        (previous.latestActivityView?.id !== stack.latestActivityView?.id &&
          stack.latestActivityView?.eventType?.startsWith('StackDrift'))
      ) {
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
