import { PlatformCapabilities, SwarmSecretView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

const selectSecrets = (inventory: SwarmInventoryUpdate) => inventory.secrets.items;

export type SwarmSecretInfoView = SwarmSecretView & {
  platformId: string;
  description: null;
  status: boolean;
  capabilities?: PlatformCapabilities | null;
};

export const useSecretsGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmSecrets', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmSecrets', args, query, selectSecrets);
  return {
    items,
    capabilities: query.data?.data.capabilities,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};

export const useSecretInfoGroup = (platformId: string, resourceId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmSecret', args);
  const secret = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmSecret',
    args,
    `/platforms/${platformId}/secrets`,
    query,
    selectSecrets,
  );
  const resource = useMemo<SwarmSecretInfoView | undefined>(
    () =>
      secret
        ? {
            ...secret,
            platformId,
            description: null,
            status: secret.inUse,
            capabilities:
              secret.capabilities ??
              (currentPlatform?.id === platformId ? (currentPlatform.capabilities ?? null) : null),
          }
        : undefined,
    [currentPlatform, platformId, secret],
  );
  return {
    resource,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};
