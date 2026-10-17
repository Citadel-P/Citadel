import { useCallback, useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useRead } from '@/lib/hooks';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';

// One catalog and, only while focused, one inspection. Existing daemon snapshots
// repair reconnects; bursts coalesce and pending refreshes die with their scope.
export function useNetworkWorkloads(platformId: string, containerId?: string, includeCatalog = true) {
  const client = useQueryClient();
  const metadata = useRef(new Map<string, string>());
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const catalog = useRead('listNetworkContainers', { id: platformId }, { staleTime: 30_000, enabled: includeCatalog });
  const networking = useRead(
    'getContainerNetworking',
    { id: containerId },
    { enabled: !!containerId, staleTime: 30_000 },
  );
  const invalidate = useCallback(() => {
    if (timer.current) return;
    timer.current = setTimeout(() => {
      timer.current = undefined;
      if (includeCatalog)
        void client.invalidateQueries({ queryKey: ['listNetworkContainers', { id: platformId }], exact: true });
      if (containerId)
        void client.invalidateQueries({ queryKey: ['getContainerNetworking', { id: containerId }], exact: true });
    }, 250);
  }, [client, platformId, containerId, includeCatalog]);
  useEffect(
    () => () => {
      clearTimeout(timer.current);
      timer.current = undefined;
    },
    [invalidate],
  );
  useDockerDaemonGroup(platformId, {
    onNetworkEvent: invalidate,
    onNetworkUsageChanged: (id) => {
      if (id === platformId) invalidate();
    },
    onContainerEvent: ({ container, eventType }) => {
      if (container.platformId !== platformId) return;
      // Health/stat changes do not alter network relationships or picker identity.
      const signature = JSON.stringify([
        container.name,
        container.containerId,
        container.dockerNodeId,
        container.deploymentId,
        container.stackId,
      ]);
      if (eventType === 'destroy') {
        metadata.current.delete(container.id);
        invalidate();
      } else if (metadata.current.get(container.id) !== signature) {
        if (metadata.current.size >= 500) metadata.current.delete(metadata.current.keys().next().value!);
        metadata.current.set(container.id, signature);
        invalidate();
      }
    },
    onContainerStateChange: (patches) => {
      // Start/stop can change endpoints and published bindings even if a shared
      // network's aggregate usage is unchanged. Inspect only the focused UUID.
      if (containerId && patches.some((patch) => patch.id === containerId && patch.state !== undefined)) invalidate();
    },
    onSwarmInventoryUpdated: (snapshot) => {
      if (snapshot.platformId === platformId) invalidate();
    },
    onSwarmNodeLocalResourcesUpdated: (snapshot) => {
      if (snapshot.platformId === platformId) invalidate();
    },
    onSwarmNodeAgentCoverageChanged: (id) => {
      if (id === platformId) invalidate();
    },
  });
  return { catalog, networking };
}
