import { PlatformView } from '@/api/generated/api.types';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';

export const { dropdown: PlatformDropdownActions, info: PlatformInfoActions } = createActionsBuilder<PlatformView>()
  .addAction({
    key: 'inspect',
    type: 'command',
    icon: SearchCode,
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      return {
        canExecute: true,
        isPending: false,
        run: () => {
          if (!selected) return;
          navigate(`/platforms/edit/${selected?.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deletePlatforms',
    canExecute: () => true,
    separatorBefore: true,
    confirm: true,
    destructive: true,
    resourceType: 'Platform',
    useVariables: (resources) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        ids: selected.map((x) => x.id!),
      };
    },
  })
  .build();
