import { canDeleteNetwork } from '../network-presentation';
import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { NetworkView } from '@/api/generated/api.types';

export const { info: NetworkInfoActions } = createActionsBuilder<NetworkView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteNetworks',
    confirm: true,
    destructive: true,
    resourceType: 'Network',
    canExecute: (r) => {
      return Array.isArray(r) ? r.every(canDeleteNetwork) : canDeleteNetwork(r);
    },
    useVariables: (resource) => {
      const { currentPlatform } = useAppContext();
      return {
        data: {
          platformId: currentPlatform?.id ?? '',
          ids: Array.isArray(resource) ? resource.map((r) => r.id) : [resource.id],
        },
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();
      return () => {
        navigate(`/platforms/${currentPlatform?.id}/networks`);
      };
    },
  })
  .build();
