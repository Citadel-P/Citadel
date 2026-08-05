import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

const selectConfigs = (inventory: SwarmInventoryUpdate) => inventory.configs.items;

export const useConfigsGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmConfigs', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmConfigs', args, query, selectConfigs);
  return { items, isLoading: query.isLoading };
};

export const useConfigInfoGroup = (platformId: string, resourceId: string) => {
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmConfig', args);
  const resource = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmConfig',
    args,
    `/platforms/${platformId}/configs`,
    query,
    selectConfigs,
  );
  return { resource, isLoading: query.isLoading, error: query.error };
};
