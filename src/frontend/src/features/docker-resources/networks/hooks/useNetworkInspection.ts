import { useCallback, useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { NetworkView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';

// Mounted only while a single details panel is open. Daemon notifications
// coalesce into one selected-network inspection, with no polling or row fan-out.
export function useNetworkInspection(network: NetworkView, platformId?: string) {
  const client = useQueryClient();
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const canInspect = !!platformId && hasCapability(network, 'canInspect');
  const query = useRead(
    'inspectNetwork',
    {
      platformId,
      networkId: network.id,
      query: { dockerNodeId: network.dockerNodeId ?? undefined },
    },
    { enabled: canInspect, staleTime: 30_000 },
  );
  const invalidate = useCallback(() => {
    if (timer.current || !canInspect) return;
    timer.current = setTimeout(() => {
      timer.current = undefined;
      void client.invalidateQueries({
        queryKey: [
          'inspectNetwork',
          {
            platformId,
            networkId: network.id,
            query: { dockerNodeId: network.dockerNodeId ?? undefined },
          },
        ],
        exact: true,
      });
    }, 250);
  }, [client, platformId, network.id, network.dockerNodeId, canInspect]);
  useEffect(
    () => () => {
      clearTimeout(timer.current);
      timer.current = undefined;
    },
    [invalidate],
  );
  useDockerDaemonGroup(canInspect ? platformId : undefined, {
    onContainerStateChange: (patches) => {
      const entries = query.data?.data?.usage?.entries ?? network.usage?.entries ?? [];
      if (patches.some((patch) => patch.state !== undefined && entries.some((entry) => entry.containerId === patch.id)))
        invalidate();
    },
    onContainerEvent: ({ container }) => {
      const entries = query.data?.data?.usage?.entries ?? network.usage?.entries ?? [];
      if (entries.some((entry) => entry.containerId === container.id)) invalidate();
    },
    onNetworkUsageChanged: (id) => {
      if (id === platformId) invalidate();
    },
    onSwarmInventoryUpdated: (snapshot) => {
      if (snapshot.platformId === platformId) invalidate();
    },
    onNetworkEvent: (event) => {
      if (!network.dockerNodeId && (event.actorId === network.id || event.network?.id === network.id)) invalidate();
    },
    onSwarmNodeLocalResourcesUpdated: (snapshot) => {
      if (network.dockerNodeId && snapshot.platformId === platformId && snapshot.networks) invalidate();
    },
    onSwarmNodeAgentCoverageChanged: (id) => {
      if (id === platformId) invalidate();
    },
  });
  return query;
}
