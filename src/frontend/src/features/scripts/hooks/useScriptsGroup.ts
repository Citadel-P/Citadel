import { AuthorizedScript, ResourceCapabilitiesView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

export const useScriptsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listScripts', readArgs);
  const [scripts, setScripts] = useState<AuthorizedScript[] | undefined>();
  const [capabilities, setCapabilities] = useState<ResourceCapabilitiesView | undefined>();
  const lastFetchedRef = useRef<AuthorizedScript[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.scripts;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setScripts(newBase);
      setCapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (script: AuthorizedScript) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((script.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const handleScriptInfoUpdated = useCallback(
    (script: AuthorizedScript, scriptName: string) => {
      setScripts((prev) => {
        if (!prev) return prev;
        if (scriptName === 'create') {
          return matchesActiveFilters(script) ? [...prev, script] : prev;
        }
        if (scriptName === 'delete') {
          return prev.filter((item) => item.id !== script.id);
        }

        const index = prev.findIndex((item) => item.id === script.id);
        if (!matchesActiveFilters(script)) {
          return index === -1 ? prev : prev.filter((item) => item.id !== script.id);
        }

        if (index === -1) return [...prev, script];

        const updated = [...prev];
        updated[index] = script;
        return updated;
      });
    },
    [matchesActiveFilters],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ScriptInfoUpdated', handleScriptInfoUpdated);
    },
    [handleScriptInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ScriptInfoUpdated', handleScriptInfoUpdated);
    },
    [handleScriptInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'scripts',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, scripts, isLoading, capabilities, selectedTagNames };
};
