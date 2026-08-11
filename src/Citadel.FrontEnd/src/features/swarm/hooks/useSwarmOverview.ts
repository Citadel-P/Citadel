import { StackReleaseStatus, SwarmOverviewView, SwarmQuorumState, SwarmQuorumView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { getServiceAvailability } from '@/features/swarm-resources/shared';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

type CachedResponse<T> = { data: T };

const count = (value: number | string) => Number(value) || 0;

export const calculateSwarmQuorum = (nodes: SwarmInventoryUpdate['nodes']['items']): SwarmQuorumView => {
  const managers = nodes.filter((node) => node.role.toLowerCase() === 'manager');
  const requiredManagers = managers.length === 0 ? 0 : Math.floor(managers.length / 2) + 1;
  const reachableManagers = managers.filter(
    (node) => !node.isStale && node.reachability.toLowerCase() === 'reachable',
  ).length;
  const hasLeader = managers.some((node) => !node.isStale && node.isLeader);
  const managerInventoryStale = managers.some((node) => node.isStale);
  const state =
    managers.length === 0 || managerInventoryStale
      ? SwarmQuorumState.Unknown
      : !hasLeader || reachableManagers < requiredManagers
        ? SwarmQuorumState.Lost
        : reachableManagers < managers.length
          ? SwarmQuorumState.Degraded
          : SwarmQuorumState.Healthy;

  return { state, reachableManagers, requiredManagers, hasLeader };
};

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
  const managerCount = inventory.nodes.items.filter((node) => node.role.toLowerCase() === 'manager').length;
  const quorum = calculateSwarmQuorum(inventory.nodes.items);
  const health = isStale ? 'Stale' : quorum.state === SwarmQuorumState.Healthy ? 'Healthy' : 'Degraded';
  const message = isStale
    ? 'The latest inventory refresh failed. Last-known inventory may be out of date.'
    : quorum.state === SwarmQuorumState.Lost
      ? `Swarm manager quorum is unavailable: ${quorum.reachableManagers} of ${managerCount} managers are reachable and ${quorum.requiredManagers} are required.`
      : quorum.state === SwarmQuorumState.Degraded
        ? `Swarm manager quorum is available, but only ${quorum.reachableManagers} of ${managerCount} managers are reachable.`
        : quorum.state === SwarmQuorumState.Unknown
          ? 'Swarm manager quorum cannot be confirmed from the current Node inventory.'
          : null;
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
    managerCount,
    quorum,
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
