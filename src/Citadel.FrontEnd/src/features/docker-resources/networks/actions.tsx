import { DockerNetworkResultView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';

export const { dropdown: NetworkDropdownActions, group: NetworkGroupActions } =
  createActionsBuilder<DockerNetworkResultView>()
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
            navigate(`/platforms/${currentPlatform?.id}/networks/${selected.id}/`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteNetworks',
      canExecute: (r) => {
        const can = (x: DockerNetworkResultView) => x.inUse === false;
        return Array.isArray(r) ? r.every(can) : can(r);
      },
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Network',
      useVariables: (resources) => {
        const { currentPlatform } = useAppContext();
        const selected = Array.isArray(resources) ? resources : [resources];
        return {
          platformId: currentPlatform?.id ?? '',
          ids: selected.map((x) => x.id!),
        };
      },
    })
    .build();
