import { StackReleaseStatus, SwarmOverviewView, SwarmQuorumState, SwarmQuorumView } from '@/api/generated/api.types';
import {
  SwarmInventoryUpdate,
  SwarmNodeLocalResourcesUpdate,
  useDockerDaemonGroup,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { getServiceAvailability } from '@/features/swarm-resources/shared';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';

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
    ? 'Some inventory is out of date. Last-known state is shown.'
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
    networkCount: inventory.networks.items.length + count(previous?.localNetworkCount ?? 0),
    localNetworkCount: count(previous?.localNetworkCount ?? 0),
    volumeCount: count(previous?.volumeCount ?? 0),
    imageCount: count(previous?.imageCount ?? 0),
    capabilities: previous?.capabilities ?? inventory.services.capabilities,
  };
};

export const useSwarmOverview = (platformId?: string) => {
  const args = useMemo(() => ({ platformId: platformId ?? '' }), [platformId]);
  const query = useRead('getSwarmOverview', args, { enabled: Boolean(platformId) });
  const queryClient = useQueryClient();
  const [localSnapshot, setLocalSnapshot] = useState<SwarmNodeLocalResourcesUpdate>();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (!platformId || inventory.platformId !== platformId) return;
      const queryKey = ['getSwarmOverview', args] as const;
      const previous = queryClient.getQueryData<CachedResponse<SwarmOverviewView>>(queryKey);
      // Cancel the older read before publishing the snapshot, so cancellation
      // neither reverts the new data nor leaves the query in an error state.
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, {
        ...previous,
        data: applySwarmInventoryToOverview(inventory, previous?.data),
      });
    },
    [args, platformId, queryClient],
  );

  const onSwarmNodeLocalResourcesUpdated = useCallback(
    (snapshot: SwarmNodeLocalResourcesUpdate) => {
      if (!platformId || snapshot.platformId !== platformId) return;
      setLocalSnapshot((previous) => ({
        ...previous, ...snapshot,
        nodeOnly: snapshot.volumes ? !!snapshot.nodeOnly : previous?.nodeOnly,
        volumeCount: snapshot.volumes ? snapshot.volumeCount : previous?.volumeCount,
      }));
      const queryKey = ['getSwarmOverview', args] as const;
      const previous = queryClient.getQueryData<CachedResponse<SwarmOverviewView>>(queryKey);
      // Node-local data alone cannot replace the initial cluster overview read.
      if (!previous?.data) return;
      const clusterNetworkCount = Math.max(
        0,
        count(previous.data.networkCount) - count(previous.data.localNetworkCount),
      );
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, {
        ...previous,
        data: {
          ...previous.data,
          imageCount: (snapshot.images?.length ?? previous.data.imageCount),
          volumeCount: (snapshot.volumeCount ?? (!snapshot.nodeOnly ? snapshot.volumes?.length : undefined) ?? previous.data.volumeCount),
          localNetworkCount: (snapshot.networks?.length ?? count(previous.data.localNetworkCount)),
          networkCount: clusterNetworkCount + (snapshot.networks?.length ?? count(previous.data.localNetworkCount)),
        },
      });
    },
    [args, platformId, queryClient],
  );

  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated, onSwarmNodeLocalResourcesUpdated });

  const overview = useMemo(() => {
    const current = query.data?.data;
    if (!current || !localSnapshot) return current;
    const clusterNetworkCount = Math.max(0, count(current.networkCount) - count(current.localNetworkCount));
    return {
      ...current,
      imageCount: (localSnapshot.images?.length ?? current.imageCount),
      volumeCount: (localSnapshot.volumeCount ?? (!localSnapshot.nodeOnly ? localSnapshot.volumes?.length : undefined) ?? current.volumeCount),
      localNetworkCount: (localSnapshot.networks?.length ?? count(current.localNetworkCount)),
      networkCount: clusterNetworkCount + (localSnapshot.networks?.length ?? count(current.localNetworkCount)),
    };
  }, [localSnapshot, query.data?.data]);

  return { ...query, overview };
};
