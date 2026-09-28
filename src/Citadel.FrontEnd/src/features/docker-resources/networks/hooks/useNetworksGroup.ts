import { useEffect, useState, useCallback, useRef } from 'react';
import { NetworksView } from '@/api/generated/api.types';
import {
  useDockerDaemonGroup,
  NetworkEvent,
  SwarmNodeLocalResourcesUpdate,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useNetworksGroup = (platformId?: string) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('listNetworks', { platformId });
  const [networks, setNetworks] = useState<NetworksView | undefined>(data?.data);
  const lastDataRef = useRef<NetworksView | undefined>(data?.data);
  const nodeSnapshotRef = useRef<SwarmNodeLocalResourcesUpdate>(undefined);
  const capabilities = data?.data.capabilities;

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      const snapshot = nodeSnapshotRef.current;
      const clusterNetworks = snapshot ? data.data.networks.filter((network) => !network.dockerNodeId) : [];
      setNetworks(
        snapshot ? { ...data.data, networks: [...clusterNetworks, ...(snapshot.networks ?? [])] } : data.data,
      );
    }
  }, [data?.data]);

  const onNetworkEvent = useCallback((event: NetworkEvent) => {
    setNetworks((prev) => {
      if (!prev?.networks) return prev;

      const { network, eventType, actorId } = event;
      const existingIndex = prev.networks.findIndex((n) => n.id === actorId && !n.dockerNodeId);

      switch (eventType) {
        case 'create':
          if (existingIndex === -1) {
            return {
              ...prev,
              networks: [network, ...prev.networks],
            };
          }
          return {
            ...prev,
            networks: prev.networks.map((value, index) => (index === existingIndex ? network : value)),
          };

        case 'destroy':
          if (existingIndex !== -1) {
            return {
              ...prev,
              networks: prev.networks.filter((n) => n.id !== actorId || !!n.dockerNodeId),
            };
          }
          break;

        default:
          break;
      }

      return prev;
    });
  }, []);

  const onSwarmNodeLocalResourcesUpdated = useCallback(
    (snapshot: SwarmNodeLocalResourcesUpdate) => {
      if (snapshot.platformId !== platformId || !snapshot.networks) return;
      const values = snapshot.networks;
      nodeSnapshotRef.current = snapshot;
      setNetworks((current) => {
        if (!current) return current;
        const clusterNetworks = current.networks.filter((network) => !network.dockerNodeId);
        return { ...current, networks: [...clusterNetworks, ...values] };
      });
    },
    [platformId],
  );

  useDockerDaemonGroup(platformId, { onNetworkEvent, onSwarmNodeLocalResourcesUpdated });

  return { error, refetch, isFetching, networks, isLoading, capabilities };
};
