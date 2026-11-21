import { useCallback, useMemo } from 'react';
import { replaceEqualDeep, useQueryClient } from '@tanstack/react-query';
import type { VolumesResponse } from '@/api/generated/api.types';
import type { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

type VolumeUpdate = { platformId: string; volumes: VolumesResponse['volumes'] } & (
  | { scope: 'all'; capabilities: VolumesResponse['capabilities'] }
  | { scope: 'manager' | 'nodes' }
  | { scope: 'one'; id: string }
);

type PartialUpdate = Exclude<VolumeUpdate, { scope: 'all' }>;
type List = ResourceResponse<'listVolumes'>;

function mergeVolumes(current: List, update: PartialUpdate): List {
  const previous = current.data.volumes;
  let volumes: VolumesResponse['volumes'];
  if (update.scope === 'nodes') volumes = [...previous.filter((v) => !v.dockerNodeId), ...update.volumes];
  else if (update.scope === 'one') {
    const matches = (v: VolumesResponse['volumes'][number]) => !v.dockerNodeId && v.id === update.id;
    volumes = previous.some(matches)
      ? previous.flatMap((v) => (matches(v) ? update.volumes : [v]))
      : [...update.volumes, ...previous];
  } else volumes = [...update.volumes, ...previous.filter((v) => v.dockerNodeId)];
  return { ...current, data: { ...current.data, volumes } };
}

export const useVolumesGroup = (platformId?: string) => {
  const queryClient = useQueryClient();
  const { key, pending } = useMemo(
    () => ({
      key: ['listVolumes', { platformId }] as const,
      pending: new Map<string, PartialUpdate>(),
    }),
    [platformId],
  );
  const { data, isLoading, error, refetch, isFetching } = useRead(
    'listVolumes',
    { platformId },
    {
      structuralSharing: (previous, incoming) => {
        let baseline = incoming as List;
        for (const update of pending.values()) baseline = mergeVolumes(baseline, update);
        pending.clear();
        return replaceEqualDeep(previous, baseline);
      },
    },
  );
  const onUpdate = useCallback(
    (update: VolumeUpdate) => {
      if (!platformId || update.platformId !== platformId) return;
      const current = queryClient.getQueryData<List>(key);
      let next: List;
      if (update.scope === 'all') {
        next = { ...current, data: { volumes: update.volumes, capabilities: update.capabilities } } as List;
      } else if (current) {
        next = mergeVolumes(current, update);
      } else {
        // Keep the initial HTTP read for capabilities and the other scope.
        // A manager snapshot supersedes all earlier manager-volume patches.
        if (update.scope === 'manager') {
          for (const [id, patch] of pending) if (patch.scope === 'one') pending.delete(id);
        }
        pending.set(update.scope === 'one' ? `one:${update.id}` : update.scope, update);
        return;
      }
      pending.clear();
      // cancelQueries reverts an active fetch before its promise settles.
      // Build from the snapshot captured above so rollback cannot erase a patch.
      void queryClient.cancelQueries({ queryKey: key, exact: true });
      queryClient.setQueryData(key, next);
    },
    [platformId, key, pending, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('VolumesUpdated', onUpdate),
    [onUpdate],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('VolumesUpdated', onUpdate),
    [onUpdate],
  );
  useRealtimeGroup({
    groupName: `volumes:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });
  return { error, refetch, isFetching, volumes: data?.data, isLoading, capabilities: data?.data.capabilities };
};
