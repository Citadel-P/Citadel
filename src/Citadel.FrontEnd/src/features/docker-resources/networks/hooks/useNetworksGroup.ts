import { useEffect, useState } from 'react';
import { NetworksView } from '@/api/generated/api.types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useNetworksGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listNetworks', { platformId });
  const [networks, setNetworks] = useState<NetworksView | undefined>();
  const { networkEvent } = useDockerDaemonGroup(platformId);

  useEffect(() => {
    if (data?.data) {
      setNetworks(data.data);
    }
  }, [data]);

  useEffect(() => {
    setNetworks((prev) => {
      if (!prev) return;

      const updatedNetworks = [...(prev.networks ?? [])];
      const existingIndex = updatedNetworks.findIndex((n) => n.id === networkEvent?.actorId);

      switch (networkEvent?.eventType) {
        case 'destroy':
          if (existingIndex !== -1) {
            updatedNetworks.splice(existingIndex, 1);
            return { ...prev, networks: updatedNetworks };
          }
          break;
        case 'create':
          return { ...prev, networks: [networkEvent.network, ...updatedNetworks] };
        default:
          break;
      }

      return prev;
    });
  }, [networkEvent]);

  return { networks, isLoading };
};
