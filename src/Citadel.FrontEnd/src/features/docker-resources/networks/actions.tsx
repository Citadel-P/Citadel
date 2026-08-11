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
            const nodeQuery = selected.dockerNodeId
              ? `?dockerNodeId=${encodeURIComponent(selected.dockerNodeId)}`
              : '';
            navigate(`/platforms/${currentPlatform?.id}/networks/${selected.id}/${nodeQuery}`);
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
        const can = (x: DockerNetworkResultView) => !x.isSystem && x.inUse === false && !x.dockerNodeId;
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
