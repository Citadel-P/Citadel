import { useEffect, useState } from 'react';
import { VolumesView } from '@/api/generated/api.types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';

export const useVolumesGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listVolumes', { platformId });
  const [volumes, setVolumes] = useState<VolumesView | undefined>();
  const { volumeEvent } = useDockerDaemonGroup(platformId);

  useEffect(() => {
    if (data?.data) {
      setVolumes(data.data);
    }
  }, [data]);

  useEffect(() => {
    setVolumes((prev) => {
      if (!prev) return;

      const updatedVolumes = [...(prev.volumes ?? [])];
      const existingIndex = updatedVolumes.findIndex((v) => v.id === volumeEvent?.actorId);

      switch (volumeEvent?.eventType) {
        case 'destroy':
          if (existingIndex !== -1) {
            updatedVolumes.splice(existingIndex, 1);
            return { ...prev, volumes: updatedVolumes };
          }
          break;
        case 'create':
          return { ...prev, volumes: [volumeEvent.volume, ...updatedVolumes] };
        default:
          break;
      }

      return prev;
    });
  }, [volumeEvent]);

  return { volumes, isLoading };
};
