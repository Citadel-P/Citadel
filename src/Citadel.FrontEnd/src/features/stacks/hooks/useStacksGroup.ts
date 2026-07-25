import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { StackView, ResourceCapabilities } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useStacksGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { tags?: string[]; platformId?: string } = {};
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return Object.keys(query).length > 0 ? { query } : undefined;
  }, [selectedPlatformId, selectedTagNames]);
  const { data, isLoading } = useRead('listStacks', readArgs);
  const [stacks, setStacks] = useState<StackView[] | undefined>();
  const [capabilities, setcapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<StackView[]>([]);
  const deletedStackIdsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!data) return;
    const newBase = data.data.stacks;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setStacks(newBase.filter((stack) => !deletedStackIdsRef.current.has(stack.id)));
      setcapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (stack: StackView) => {
      if (selectedPlatformId && stack.platformId !== selectedPlatformId) return false;
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((stack.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedPlatformId, selectedTagNames],
  );

  const handleStackInfoUpdated = useCallback((stack: StackView, action: string) => {
    setStacks((prev) => {
      if (!prev) return prev;
      if (action === 'create') {
        deletedStackIdsRef.current.delete(stack.id);
        const index = prev.findIndex((current) => current.id === stack.id);
        if (!matchesActiveFilters(stack)) {
          return index === -1 ? prev : prev.filter((current) => current.id !== stack.id);
        }
        if (index === -1) {
          return [...prev, stack];
        }

        const updated = [...prev];
        updated[index] = stack;
        return updated;
      }
      if (action === 'delete') {
        deletedStackIdsRef.current.add(stack.id);
        return prev.filter((d) => d.id !== stack.id);
      }
      if (deletedStackIdsRef.current.has(stack.id)) {
        return prev;
      }

      const index = prev.findIndex((d) => d.id === stack.id);
      if (!matchesActiveFilters(stack)) {
        return index === -1 ? prev : prev.filter((d) => d.id !== stack.id);
      }

      if (index !== -1) {
        const updated = [...prev];
        updated[index] = stack;
        return updated;
      }
      return [...prev, stack];
    });
  }, [matchesActiveFilters]);

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

  return { stacks, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
