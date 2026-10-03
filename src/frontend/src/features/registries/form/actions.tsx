import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { AuthorizedRegistryView } from '@/api/generated/api.types';

export const { info: RegistryActions } = createActionsBuilder<AuthorizedRegistryView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteRegistries',
    confirm: true,
    destructive: true,
    resourceType: 'Registry',
    canExecute: () => true,
    useVariables: (resource) => {
      return {
        ids: [(resource as AuthorizedRegistryView).id],
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/registries`);
      };
    },
  })
  .build();
