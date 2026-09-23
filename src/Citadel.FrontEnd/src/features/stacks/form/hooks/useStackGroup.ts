import { useMemo, useState, useCallback } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useQueryClient } from '@tanstack/react-query';
import { StackView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useStackGroup = (stackId: string) => {
  const { data, isLoading } = useRead('getStack', { stackId });
  const queryClient = useQueryClient();
  const [stackUpdate, setStackUpdate] = useState<Partial<StackView> | null>(null);

  const stack = useMemo(() => {
    if (!data?.data) return undefined;
    if (!stackUpdate) return data.data;
    return { ...data.data, ...stackUpdate };
  }, [data, stackUpdate]);

  const handleStackInfoUpdated = useCallback(
    (stack: StackView) => {
      if (stack.id !== stackId) return;
      const activityInfo = stack.latestActivityView?.info;
      const info = Array.isArray(activityInfo) ? { ...activityInfo[1], $type: activityInfo[0] } : activityInfo;

      setStackUpdate({
        name: stack.name,
        status: stack.status,
        description: stack.description,
        controlState: stack.controlState,
        driftPolicy: stack.driftPolicy,
        stackUpdateState: stack.stackUpdateState,
        platformStatus: stack.platformStatus,
        platformName: stack.platformName,
        latestActivityView: stack.latestActivityView ? { ...stack.latestActivityView, info } : null,
      });

      queryClient.invalidateQueries({ queryKey: ['getStackDrift', { stackId: stack.id }] });
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

  return { stack, isLoading };
};
