import { AutomationActionView, ResourceCapabilities } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

export const useAutomationActionsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listAutomationActions', readArgs);
  const [actions, setActions] = useState<AutomationActionView[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<AutomationActionView[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.actions;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setActions(newBase);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (action: AutomationActionView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((action.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const handleAutomationActionInfoUpdated = useCallback(
    (action: AutomationActionView, actionName: string) => {
      setActions((prev) => {
        if (!prev) return prev;
        if (actionName === 'create') {
          return matchesActiveFilters(action) ? [...prev, action] : prev;
        }
        if (actionName === 'delete') {
          return prev.filter((item) => item.id !== action.id);
        }

        const index = prev.findIndex((item) => item.id === action.id);
        if (!matchesActiveFilters(action)) {
          return index === -1 ? prev : prev.filter((item) => item.id !== action.id);
        }

        if (index === -1) return [...prev, action];

        const updated = [...prev];
        updated[index] = action;
        return updated;
      });
    },
    [matchesActiveFilters],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('AutomationActionInfoUpdated', handleAutomationActionInfoUpdated);
    },
    [handleAutomationActionInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('AutomationActionInfoUpdated', handleAutomationActionInfoUpdated);
    },
    [handleAutomationActionInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'automation-actions',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, actions, isLoading, capabilities, selectedTagNames };
};
