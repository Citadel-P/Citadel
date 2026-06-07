import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { StackView, ResourceCapabilities } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useStacksGroup = () => {
  const { data, isLoading } = useRead('listStacks');
  const [stacks, setStacks] = useState<StackView[] | undefined>();
  const [capabilities, setcapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<StackView[]>([]);

  useEffect(() => {
    if (!data) return;
    const newBase = data.data.stacks;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setStacks(newBase);
      setcapabilities(data.data.capabilities)
    }
  }, [data]);

  const handleStackInfoUpdated = useCallback((stack: StackView, action: string) => {
    setStacks((prev) => {
      if (!prev) return prev;

      if (action === 'create') {
        return [...prev, stack];
      }
      if (action === 'delete') {
        return prev.filter((d) => d.id !== stack.id);
      }

      const index = prev.findIndex((d) => d.id === stack.id);
      if (index !== -1) {
        const updated = [...prev];
        updated[index] = stack;
        return updated;
      }
      return prev;
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
    groupName: 'stacks',
    setupEventListeners,
    removeEventListeners,
  });

  return { stacks, isLoading, capabilities };
};
