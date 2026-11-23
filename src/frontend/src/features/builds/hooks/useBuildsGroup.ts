import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { AuthorizedProject } from '@/api/generated/api.types';
import { useResourceTagFilter } from '@/features/tags/components';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
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
  const query = useRead('listBuildProjects', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;

  const projects = useCollectionSubscription({
    groupName,
    eventName: 'BuildProjectInfoUpdated',
    enabled: filterReady,
    field: 'projects',
    query,
    merge: mergeRun,
  });
  return { error, refetch, isFetching, projects: projects ?? [], isLoading, capabilities, selectedTagNames };
};
