import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { ScriptSummaryView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

export const useScriptsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = 'scripts';
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listScripts', readArgs, {
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;
  const matchesActiveFilters = useCallback(
    (script: ScriptSummaryView) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((script.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.scripts : undefined;
  }, [refetch]);
  const [scripts, handleScriptInfoUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.scripts,
    JSON.stringify(readArgs ?? {}),
    matchesActiveFilters,
    reload,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ScriptSummaryUpdated', handleScriptInfoUpdated);
    },
    [handleScriptInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ScriptSummaryUpdated', handleScriptInfoUpdated);
    },
    [handleScriptInfoUpdated],
  );

  useRealtimeGroup({
    groupName,
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, scripts, isLoading, capabilities, selectedTagNames };
};
