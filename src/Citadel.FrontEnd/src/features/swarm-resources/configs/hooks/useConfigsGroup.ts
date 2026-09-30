import type { PlatformCapabilities, SwarmConfigView } from '@/api/generated/api.types';
import type { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

const selectConfigs = (inventory: SwarmInventoryUpdate) => inventory.configs.items;

export type SwarmConfigInfoView = SwarmConfigView & {
  platformId: string;
  description: null;
  status: boolean;
  capabilities?: PlatformCapabilities | null;
};

export const useConfigsGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmConfigs', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmConfigs', args, query, selectConfigs);
  return {
    items,
    capabilities: query.data?.data.capabilities,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};

export const useConfigInfoGroup = (platformId: string, resourceId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmConfig', args);
  const config = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmConfig',
    args,
    `/platforms/${platformId}/configs`,
    query,
    selectConfigs,
  );
  const resource = useMemo<SwarmConfigInfoView | undefined>(
    () =>
      config
        ? {
            ...config,
            platformId,
            description: null,
            status: config.inUse,
            capabilities:
              config.capabilities ??
              (currentPlatform?.id === platformId ? (currentPlatform.capabilities ?? null) : null),
          }
        : undefined,
    [config, currentPlatform, platformId],
  );
  return {
    resource,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};
