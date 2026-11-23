import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

export const useScriptsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = collectionGroup('scripts', selectedTagNames);
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listScripts', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.scripts : undefined;
  }, [refetch]);
  const [scripts, handleScriptInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.scripts,
    JSON.stringify(readArgs ?? {}),
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
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, scripts, isLoading, capabilities, selectedTagNames };
};
