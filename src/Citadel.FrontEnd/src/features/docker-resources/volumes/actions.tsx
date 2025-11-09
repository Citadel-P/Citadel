import { DockerVolumeResult } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';

export const { dropdown: VolumeDropdownActions, group: VolumeGroupActions } = createActionsBuilder<DockerVolumeResult>()
  .addAction({
    key: 'inspect',
    type: 'command',
    icon: SearchCode,
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
          navigate(`/platforms/${currentPlatform?.id}/volumes/${selected.id}/`);
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteVolumes',
    invalidate: 'listVolumes',
    canExecute: (r) => {
      const can = (x: DockerVolumeResult) => x.inUse === false;
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
