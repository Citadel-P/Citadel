import { StackReleaseStatus, SwarmOverviewView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { getServiceAvailability } from '@/features/swarm-resources/shared';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

type CachedResponse<T> = { data: T };

const count = (value: number | string) => Number(value) || 0;

export const applySwarmInventoryToOverview = (
  inventory: SwarmInventoryUpdate,
  previous?: SwarmOverviewView,
): SwarmOverviewView => {
  const isStale =
    inventory.nodes.items.some((item) => item.isStale) ||
    inventory.services.items.some((item) => item.isStale) ||
    inventory.tasks.items.some((item) => item.isStale) ||
    inventory.networks.items.some((item) => item.isStale) ||
    inventory.secrets.items.some((item) => item.isStale) ||
    inventory.configs.items.some((item) => item.isStale);
  const health = isStale ? 'Stale' : 'Healthy';
  const message = isStale ? 'The latest inventory refresh failed. Last-known inventory may be out of date.' : null;
  const serviceStatusCounts = inventory.services.items.reduce(
    (counts, service) => {
      const status = getServiceAvailability(service).status;
      if (status === StackReleaseStatus.Healthy) counts.healthy++;
      else if (status === StackReleaseStatus.Degraded) counts.degraded++;
      else if (status === StackReleaseStatus.Failed) counts.failed++;
      else if (status === StackReleaseStatus.Stopped) counts.stopped++;
      else counts.unknown++;
      return counts;
    },
    {
      total: inventory.services.items.length,
      healthy: 0,
      degraded: 0,
      failed: 0,
      stopped: 0,
      paused: 0,
      inProgress: 0,
      unknown: 0,
    },
  );

  return {
    platformId: inventory.platformId,
    health,
    message,
    isStale,
    nodeCount: inventory.nodes.items.length,
    managerCount: inventory.nodes.items.filter((node) => node.role.toLowerCase() === 'manager').length,
    serviceCount: inventory.services.items.length,
    serviceStatusCounts,
    runningTaskCount: inventory.services.items.reduce((total, service) => total + count(service.runningTaskCount), 0),
    desiredTaskCount: inventory.services.items.reduce((total, service) => total + count(service.desiredTaskCount), 0),
    networkCount: inventory.networks.items.length,
    capabilities: previous?.capabilities ?? inventory.services.capabilities,
  };
};

export const useSwarmOverview = (platformId?: string) => {
  const args = useMemo(() => ({ platformId: platformId ?? '' }), [platformId]);
  const query = useRead('getSwarmOverview', args, { enabled: Boolean(platformId) });
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (!platformId || inventory.platformId !== platformId) return;
      const queryKey = ['getSwarmOverview', args] as const;
      queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, (previous) => ({
        ...previous,
        data: applySwarmInventoryToOverview(inventory, previous?.data),
      }));
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
    },
    [args, platformId, queryClient],
  );

  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });

  return { ...query, overview: query.data?.data };
};
