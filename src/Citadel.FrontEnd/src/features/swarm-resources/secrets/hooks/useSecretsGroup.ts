import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

const selectSecrets = (inventory: SwarmInventoryUpdate) => inventory.secrets.items;

export const useSecretsGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmSecrets', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmSecrets', args, query, selectSecrets);
  return { items, isLoading: query.isLoading };
};

export const useSecretInfoGroup = (platformId: string, resourceId: string) => {
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmSecret', args);
  const resource = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmSecret',
    args,
    `/platforms/${platformId}/secrets`,
    query,
    selectSecrets,
  );
  return { resource, isLoading: query.isLoading, error: query.error };
};
