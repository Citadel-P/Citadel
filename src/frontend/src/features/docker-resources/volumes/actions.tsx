import { VolumeView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { VolumeBrowseDropdownAction, VolumeBrowseGroupAction } from './volume-browser-sheet';

const volumeActions = createActionsBuilder<VolumeView>()
  .addAction({
    key: 'inspect',
    type: 'command',
    icon: SearchCode,
    requiredCapabilities: ['canInspect'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      let canExecute = !!selected;
      if (Array.isArray(resources)) {
        canExecute &&= resources.length === 1;
      }
      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !selected) return;
          const nodeQuery = selected.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(selected.dockerNodeId)}` : '';
          navigate(`/platforms/${currentPlatform?.id}/volumes/${selected.id}/${nodeQuery}`);
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteVolumes',
    canExecute: (r) => {
      const can = (x: VolumeView) => x.inUse === false && !x.dockerNodeId;
      return Array.isArray(r) ? r.every(can) : can(r);
    },
    separatorBefore: true,
    confirm: true,
    destructive: true,
    resourceType: 'Volume',
    useVariables: (resources) => {
      const { currentPlatform } = useAppContext();
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        platformId: currentPlatform?.id ?? '',
        names: selected.map((x) => x.id!),
        force: true,
      };
    },
  })
  .build();

export const VolumeDropdownActions = {
  browse: VolumeBrowseDropdownAction,
  ...volumeActions.dropdown,
};

export const VolumeGroupActions = {
  browse: VolumeBrowseGroupAction,
  ...volumeActions.group,
};
