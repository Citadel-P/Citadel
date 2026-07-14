import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DockerVolumeResultView } from '@/api/generated/api.types';
import { VolumeBrowseInfoAction } from '../volume-browser-sheet';

const volumeInfoActions = createActionsBuilder<DockerVolumeResultView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteVolumes',
    confirm: true,
    destructive: true,
    resourceType: 'Volume',
    canExecute: (r) => {
      const containersCount = Array.isArray(r)
        ? r.some((x) => Object.keys(x?.containers ?? {}).length === 0)
        : Object.keys(r?.containers ?? {}).length === 0;
      return containersCount;
    },
    useVariables: (resource) => {
      const { currentPlatform } = useAppContext();
      return {
        platformId: currentPlatform?.id ?? '',
        names: Array.isArray(resource) ? resource.map((r) => r.id) : [resource.id],
        force: true,
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();

      return () => {
        navigate(`/platforms/${currentPlatform?.id}/volumes`);
      };
    },
  })
  .build();

export const VolumeInfoActions = {
  browse: VolumeBrowseInfoAction,
  ...volumeInfoActions.info,
};
