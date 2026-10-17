import { useEffect, useState, useCallback, useRef, type SetStateAction } from 'react';
import { NetworksResponse } from '@/api/generated/api.types';
import {
  useDockerDaemonGroup,
  NetworkEvent,
  SwarmNodeLocalResourcesUpdate,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useNetworksGroup = (platformId?: string, refreshUsage = false) => {
  const { data, isLoading, error, refetch, isFetching, dataUpdatedAt } = useRead('listNetworks', { platformId });
  const [scoped, setScoped] = useState({ platformId, value: data?.data });
  const networks = scoped.platformId === platformId ? scoped.value : data?.data;
  const setNetworks = useCallback(
    (update: SetStateAction<NetworksResponse | undefined>) => {
      setScoped((current) => {
        const previous = current.platformId === platformId ? current.value : undefined;
        return { platformId, value: typeof update === 'function' ? update(previous) : update };
      });
    },
    [platformId],
  );
  const lastDataRef = useRef({ platformId, value: data?.data, dataUpdatedAt });
  const nodeSnapshotRef = useRef<SwarmNodeLocalResourcesUpdate>(undefined);
  const requestSnapshot = useRef<SwarmNodeLocalResourcesUpdate>(undefined);
  const fetching = useRef(false);
  const capabilities = data?.data.capabilities;

  // A pre-request realtime snapshot must not mask a newer HTTP response. Keep
  // only snapshots received during that request, which may be ahead of its read.
  useEffect(() => {
    if (isFetching && !fetching.current) requestSnapshot.current = nodeSnapshotRef.current;
    fetching.current = isFetching;
  }, [isFetching]);

  useEffect(() => {
    if (nodeSnapshotRef.current?.platformId !== platformId) nodeSnapshotRef.current = undefined;
    if (
      data?.data &&
      (data.data !== lastDataRef.current.value ||
        dataUpdatedAt !== lastDataRef.current.dataUpdatedAt ||
        platformId !== lastDataRef.current.platformId)
    ) {
      lastDataRef.current = { platformId, value: data.data, dataUpdatedAt };
      const snapshot =
        nodeSnapshotRef.current?.platformId === platformId && nodeSnapshotRef.current !== requestSnapshot.current
          ? nodeSnapshotRef.current
          : undefined;
      if (!snapshot) nodeSnapshotRef.current = undefined;
      const clusterNetworks = snapshot ? data.data.networks.filter((network) => !network.dockerNodeId) : [];
      setNetworks(
        snapshot ? { ...data.data, networks: [...clusterNetworks, ...(snapshot.networks ?? [])] } : data.data,
      );
    }
  }, [data?.data, dataUpdatedAt, platformId, setNetworks]);

  const onNetworkEvent = useCallback(
    (event: NetworkEvent) => {
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
    },
    [setNetworks],
  );

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
    [platformId, setNetworks],
  );

  // A workload map depends on relationships across networks. Refresh one shared
  // list on invalidation, never inspect every network/container in the graph.
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const refresh = useCallback(() => {
    if (!refreshUsage || refreshTimer.current) return;
    refreshTimer.current = setTimeout(() => {
      refreshTimer.current = undefined;
      void refetch();
    }, 250);
  }, [refreshUsage, refetch]);
  useEffect(
    () => () => {
      clearTimeout(refreshTimer.current);
      refreshTimer.current = undefined;
    },
    [platformId, refresh],
  );

  useDockerDaemonGroup(platformId, {
    onNetworkEvent: (event) => {
      onNetworkEvent(event);
      refresh();
    },
    onSwarmNodeLocalResourcesUpdated,
    onNetworkUsageChanged: (id) => {
      if (id === platformId) refresh();
    },
    onSwarmInventoryUpdated: (snapshot) => {
      if (snapshot.platformId === platformId) refresh();
    },
    onSwarmNodeAgentCoverageChanged: (id) => {
      if (id === platformId) refresh();
    },
    onContainerStateChange: (patches) => {
      if (
        patches.some(
          (patch) =>
            patch.state !== undefined &&
            networks?.networks.some((network) =>
              network.usage?.entries.some((entry) => entry.containerId === patch.id),
            ),
        )
      )
        refresh();
    },
  });

  return { error, refetch, isFetching, networks, isLoading, capabilities };
};
