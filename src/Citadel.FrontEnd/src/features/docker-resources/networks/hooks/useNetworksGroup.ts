import { useEffect, useState, useCallback } from 'react';
import { NetworksView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, NetworkEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useNetworksGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listNetworks', { platformId });
  const [networks, setNetworks] = useState<NetworksView | undefined>();

  const onNetworkEvent = useCallback((event: NetworkEvent) => {
    setNetworks((prev) => {
      if (!prev?.networks) return prev;

      const { network, eventType, actorId } = event;
      const existingIndex = prev.networks.findIndex((n) => n.id === actorId);

      switch (eventType) {
        case 'create':
          if (existingIndex === -1) {
            return {
              ...prev,
              networks: [network, ...prev.networks],
            };
          }
          break;

        case 'destroy':
          if (existingIndex !== -1) {
            return {
              ...prev,
              networks: prev.networks.filter((n) => n.id !== actorId),
            };
          }
          break;

        default:
          break;
      }

      return prev;
    });
  }, []);

  useDockerDaemonGroup(platformId, { onNetworkEvent });

  useEffect(() => {
    if (data?.data) {
      setNetworks(data.data);
    }
  }, [data]);

  return { networks, isLoading };
};
