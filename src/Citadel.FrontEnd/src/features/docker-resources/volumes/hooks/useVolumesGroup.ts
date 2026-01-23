import { useEffect, useState, useCallback } from 'react';
import { VolumesView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, VolumeEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useVolumesGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listVolumes', { platformId });
  const [volumes, setVolumes] = useState<VolumesView | undefined>();

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

  useDockerDaemonGroup(platformId, { onVolumeEvent });

  useEffect(() => {
    if (data?.data) {
      setVolumes(data.data);
    }
  }, [data]);

  return { volumes, isLoading };
};
