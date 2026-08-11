import { useEffect, useState, useCallback, useRef } from 'react';
import { VolumesView } from '@/api/generated/api.types';
import {
  useDockerDaemonGroup,
  VolumeEvent,
  SwarmNodeLocalResourcesUpdate,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useVolumesGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listVolumes', { platformId });
  const [volumes, setVolumes] = useState<VolumesView | undefined>();
  const capabilities = data?.data.capabilities;

  const lastDataRef = useRef<VolumesView | undefined>(data?.data);
  const nodeSnapshotRef = useRef<SwarmNodeLocalResourcesUpdate>();

  useEffect(() => {
    if (data?.data && data.data !== lastDataRef.current) {
      lastDataRef.current = data.data;
      setVolumes(
        nodeSnapshotRef.current
          ? { ...data.data, volumes: nodeSnapshotRef.current.volumes }
          : data.data,
      );
    }
  }, [data?.data]);

  const onVolumeEvent = useCallback((event: VolumeEvent) => {
    setVolumes((prev) => {
      if (!prev?.volumes) return prev;

      const { volume, eventType, actorId } = event;
      const existingIndex = prev.volumes.findIndex((v) => v.id === actorId);

      switch (eventType) {
        case 'create':
          if (existingIndex === -1) {
            return {
              ...prev,
              volumes: [volume, ...prev.volumes],
            };
          }
          break;

        case 'destroy':
          if (existingIndex !== -1) {
            return {
              ...prev,
              volumes: prev.volumes.filter((v) => v.id !== actorId),
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
      if (snapshot.platformId !== platformId) return;
      nodeSnapshotRef.current = snapshot;
      setVolumes((current) => (current ? { ...current, volumes: snapshot.volumes } : current));
    },
    [platformId],
  );

  useDockerDaemonGroup(platformId, { onVolumeEvent, onSwarmNodeLocalResourcesUpdated });

  return { volumes, isLoading, capabilities };
};
