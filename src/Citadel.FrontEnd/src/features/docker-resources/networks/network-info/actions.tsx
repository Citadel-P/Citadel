import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DockerNetworkDetailsView } from '@/api/generated/api.types';

export const { info: NetworkInfoActions } = createActionsBuilder<DockerNetworkDetailsView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteNetworks',
    confirm: true,
    destructive: true,
    resourceType: 'Network',
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
        ids: Array.isArray(resource) ? resource.map((r) => r.id) : [resource.id],
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
