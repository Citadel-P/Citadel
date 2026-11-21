import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
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

export const useVolumesGroup = (platformId?: string) => {
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch, isFetching } = useRead('listVolumes', { platformId });
  const onUpdate = useCallback(
    (update: VolumeUpdate) => {
      if (!platformId || update.platformId !== platformId) return;
      const key = ['listVolumes', { platformId }] as const;
      if (update.scope === 'all') void queryClient.cancelQueries({ queryKey: key, exact: true });
      queryClient.setQueryData<ResourceResponse<'listVolumes'>>(key, (current) => {
        if (update.scope === 'all') {
          return {
            ...current,
            data: { volumes: update.volumes, capabilities: update.capabilities },
          } as ResourceResponse<'listVolumes'>;
        }
        if (!current) return current;
        const previous = current.data.volumes;
        let volumes: VolumesResponse['volumes'];
        if (update.scope === 'nodes') volumes = [...previous.filter((v) => !v.dockerNodeId), ...update.volumes];
        else if (update.scope === 'manager') volumes = [...update.volumes, ...previous.filter((v) => v.dockerNodeId)];
        else if (update.scope === 'one') {
          const existing = previous.some((v) => !v.dockerNodeId && v.id === update.id);
          volumes = existing
            ? previous.flatMap((v) => (!v.dockerNodeId && v.id === update.id ? update.volumes : [v]))
            : [...update.volumes, ...previous];
        } else return current;
        return { ...current, data: { ...current.data, volumes } };
      });
    },
    [platformId, queryClient],
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
