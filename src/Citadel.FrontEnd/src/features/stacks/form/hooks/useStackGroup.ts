import { useMemo, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { StackView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useStackGroup = (stackId: string) => {
  const { data, isLoading } = useRead('getStack', { stackId });
  const [stackUpdate, setStackUpdate] = useState<Partial<StackView> | null>(null);

  const stack = useMemo(() => {
    if (!data?.data) return undefined;
    if (!stackUpdate) return data.data;
    return { ...data.data, ...stackUpdate };
  }, [data, stackUpdate]);

  const handleStackInfoUpdated = useCallback((stack: StackView) => {
    const info = (stack?.latestActivityView?.info as any)?.[1];
    if (info) {
      info.$type = (stack?.latestActivityView?.info as any)?.[0];
    }

    setStackUpdate({
      name: stack.name,
      status: stack.status,
      description: stack.description,
      controlState: stack.controlState,
      platformStatus: stack.platformStatus,
      platformName: stack.platformName,
      latestActivityView: stack.latestActivityView ? { ...stack.latestActivityView, info } : null,
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('StackInfoUpdated', handleStackInfoUpdated);
    },
    [handleStackInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('StackInfoUpdated', handleStackInfoUpdated);
    },
    [handleStackInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'stack:' + stackId,
    skip: !stackId,
    setupEventListeners,
    removeEventListeners,
  });

  return { stack, isLoading };
};
