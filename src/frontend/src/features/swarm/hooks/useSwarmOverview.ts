import { SwarmOverviewView } from '@/api/generated/api.types';
import {
  SwarmInventoryUpdate,
  SwarmNodeLocalResourcesUpdate,
  useDockerDaemonGroup,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';

type CachedResponse<T> = { data: T };

const count = (value: number | string) => Number(value) || 0;

export const useSwarmOverview = (platformId?: string) => {
  const args = useMemo(() => ({ platformId: platformId ?? '' }), [platformId]);
  const query = useRead('getSwarmOverview', args, { enabled: Boolean(platformId) });
  const queryClient = useQueryClient();
  const [localSnapshot, setLocalSnapshot] = useState<SwarmNodeLocalResourcesUpdate>();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (!platformId || inventory.platformId !== platformId) return;
      setLocalSnapshot(undefined);
      const queryKey = ['getSwarmOverview', args] as const;
      const previous = queryClient.getQueryData<CachedResponse<SwarmOverviewView>>(queryKey);
      // Cancel the older read before publishing the snapshot, so cancellation
      // neither reverts the new data nor leaves the query in an error state.
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, {
        ...previous,
        data: inventory.overview,
      });
    },
    [args, platformId, queryClient],
  );

  const onSwarmNodeLocalResourcesUpdated = useCallback(
    (snapshot: SwarmNodeLocalResourcesUpdate) => {
      if (!platformId || snapshot.platformId !== platformId) return;
      if (snapshot.overview) {
        const queryKey = ['getSwarmOverview', args] as const;
        void queryClient.cancelQueries({ queryKey, exact: true });
        setLocalSnapshot(undefined);
        queryClient.setQueryData<CachedResponse<SwarmOverviewView>>(queryKey, { data: snapshot.overview });
        return;
      }
      // Partial local observations can arrive before a complete overview.
      // Retain only supplied collections, scoped to this Platform.
      setLocalSnapshot((retained) => {
        const previous = retained?.platformId === platformId ? retained : undefined;
        return {
          ...previous,
          ...snapshot,
          nodeOnly: snapshot.volumes ? !!snapshot.nodeOnly : previous?.nodeOnly,
          volumeCount:
            snapshot.volumeCount ??
            (snapshot.volumes && !snapshot.nodeOnly ? snapshot.volumes.length : previous?.volumeCount),
        };
      });
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
          imageCount: snapshot.images?.length ?? previous.data.imageCount,
          volumeCount:
            snapshot.volumeCount ??
            (!snapshot.nodeOnly ? snapshot.volumes?.length : undefined) ??
            previous.data.volumeCount,
          localNetworkCount: snapshot.networks?.length ?? count(previous.data.localNetworkCount),
          networkCount: clusterNetworkCount + (snapshot.networks?.length ?? count(previous.data.localNetworkCount)),
        },
      });
    },
    [args, platformId, queryClient],
  );

  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated, onSwarmNodeLocalResourcesUpdated });

  const overview = useMemo(() => {
    const current = query.data?.data;
    if (!current || !localSnapshot || localSnapshot.platformId !== platformId) return current;
    const clusterNetworkCount = Math.max(0, count(current.networkCount) - count(current.localNetworkCount));
    return {
      ...current,
      imageCount: localSnapshot.images?.length ?? current.imageCount,
      volumeCount:
        localSnapshot.volumeCount ??
        (!localSnapshot.nodeOnly ? localSnapshot.volumes?.length : undefined) ??
        current.volumeCount,
      localNetworkCount: localSnapshot.networks?.length ?? count(current.localNetworkCount),
      networkCount: clusterNetworkCount + (localSnapshot.networks?.length ?? count(current.localNetworkCount)),
    };
  }, [localSnapshot, platformId, query.data?.data]);

  return { ...query, overview };
};
