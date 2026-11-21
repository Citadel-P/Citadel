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
  const { selectedTagNames } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead('listBuildProjects', readArgs);
  const capabilities = data?.data.capabilities;
  const matchesActiveFilters = useCallback(
    (project: AuthorizedProject) => {
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((project.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedTagNames],
  );

  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.projects : undefined;
  }, [refetch]);
  const [projects, handleBuildProjectInfoUpdated, onJoinedGroup] = useCollectionRows(
    data?.data.projects,
    JSON.stringify(readArgs ?? {}),
    matchesActiveFilters,
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
    groupName: 'build-projects',
    onJoinedGroup,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, projects: projects ?? [], isLoading, capabilities, selectedTagNames };
};
