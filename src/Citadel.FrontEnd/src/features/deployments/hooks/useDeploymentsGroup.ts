import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { DeploymentView, ResourceCapabilities } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';

export const useDeploymentsGroup = () => {
  const { selectedTagNames } = useResourceTagFilter();
  const { selectedPlatformId } = useResourcePlatformFilter();
  const readArgs = useMemo(() => {
    const query: { tags?: string[]; platformId?: string } = {};
    if (selectedTagNames.length > 0) query.tags = selectedTagNames;
    if (selectedPlatformId) query.platformId = selectedPlatformId;
    return Object.keys(query).length > 0 ? { query } : undefined;
  }, [selectedPlatformId, selectedTagNames]);
  const { data, isLoading, error, refetch, isFetching } = useRead('listDeployments', readArgs);
  const [deployments, setDeployments] = useState<DeploymentView[] | undefined>();
  const [capabilities, setcapabilities] = useState<ResourceCapabilities | undefined>();
  const lastFetchedRef = useRef<DeploymentView[]>([]);

  useEffect(() => {
    if (!data) return;
    const newBase = data.data.deployments;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setDeployments(newBase);
      setcapabilities(data.data.capabilities);
    }
  }, [data]);

  const matchesActiveFilters = useCallback(
    (deployment: DeploymentView) => {
      if (selectedPlatformId && deployment.platformId !== selectedPlatformId) return false;
      if (selectedTagNames.length === 0) return true;

      const tagNames = new Set((deployment.tags ?? []).map((tag) => tag.name.trim().toLowerCase()));
      return selectedTagNames.every((tagName) => tagNames.has(tagName.trim().toLowerCase()));
    },
    [selectedPlatformId, selectedTagNames],
  );

  const handleDeploymentInfoUpdated = useCallback(
    (deployment: DeploymentView, action: string) => {
      setDeployments((prev) => {
        if (!prev) return prev;
        if (action === 'create') {
          return matchesActiveFilters(deployment) ? [...prev, deployment] : prev;
        }
        if (action === 'delete') {
          return prev.filter((d) => d.id !== deployment.id);
        }

        const index = prev.findIndex((d) => d.id === deployment.id);
        if (!matchesActiveFilters(deployment)) {
          return index === -1 ? prev : prev.filter((d) => d.id !== deployment.id);
        }

        if (index !== -1) {
          const updated = [...prev];
          updated[index] = deployment;
          return updated;
        }
        return [...prev, deployment];
      });
    },
    [matchesActiveFilters],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'deployments',
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, deployments, isLoading, capabilities, selectedTagNames, selectedPlatformId };
};
