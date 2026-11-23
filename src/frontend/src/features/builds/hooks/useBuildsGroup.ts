import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { AuthorizedProject } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useCallback, useMemo } from 'react';
import { selectBuildProjectLatestRun } from '../build-run-state';

const mergeRun = (incoming: AuthorizedProject, existing: AuthorizedProject) => ({
  ...incoming,
  latestRun: selectBuildProjectLatestRun(incoming, existing.latestRun),
});

export const useBuildsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = collectionGroup('build-projects', selectedTagNames);
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const { data, isLoading, error, refetch, isFetching } = useRead('listBuildProjects', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const capabilities = data?.data.capabilities;

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.projects : undefined;
  }, [refetch]);
  const [projects, handleBuildProjectInfoUpdated, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.projects,
    JSON.stringify(readArgs ?? {}),
    reload,
    mergeRun,
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
    },
    [handleBuildProjectInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BuildProjectInfoUpdated', handleBuildProjectInfoUpdated);
    },
    [handleBuildProjectInfoUpdated],
  );

  useRealtimeGroup({
    enabled: filterReady,
    groupName,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, projects: projects ?? [], isLoading, capabilities, selectedTagNames };
};
