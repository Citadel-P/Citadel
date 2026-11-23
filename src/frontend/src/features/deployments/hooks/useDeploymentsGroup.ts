import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { collectionGroup } from '@/lib/collection-scope';
import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useDeploymentsGroup = () => {
  const { selectedTagNames, filterReady } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { tags?: string[]; platformId?: string } = {};
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return Object.keys(query).length > 0 ? { query } : undefined;
  }, [selectedPlatformId, selectedTagNames]);
  const query = useRead('listDeployments', readArgs, {
    enabled: filterReady,
  });
  const groupName = collectionGroup('deployments', selectedTagNames, selectedPlatformId || undefined);
  const { error, refetch, isFetching, isLoading } = query;
  const capabilities = query.data?.data.capabilities;
  const deployments = useCollectionSubscription({
    groupName,
    eventName: 'DeploymentInfoUpdated',
    enabled: filterReady,
    field: 'deployments',
    query,
  });
  return { error, refetch, isFetching, deployments, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
