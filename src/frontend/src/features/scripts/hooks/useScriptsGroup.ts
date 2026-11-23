import { collectionGroup } from '@/lib/collection-scope';
import { useCollectionStructuralSharing } from '@/hooks/useQueryReconciliation';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { useResourceTagFilter } from '@/features/tags/components';

export const useScriptsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const readArgs = useMemo(
    () => (selectedTagNames.length > 0 ? { query: { tags: selectedTagNames } } : undefined),
    [selectedTagNames],
  );
  const groupName = collectionGroup('scripts', selectedTagNames);
  const structuralSharing = useCollectionStructuralSharing(groupName);
  const query = useRead('listScripts', readArgs, {
    enabled: filterReady,
    structuralSharing,
  });
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;

  const scripts = useCollectionSubscription({
    groupName,
    eventName: 'ScriptSummaryUpdated',
    enabled: filterReady,
    field: 'scripts',
    query,
  });
  return { error, refetch, isFetching, scripts, isLoading, capabilities, selectedTagNames };
};
