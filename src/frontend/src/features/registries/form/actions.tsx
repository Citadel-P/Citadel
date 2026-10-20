import { useRegistryDeletionEffects } from '../queries';
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
        data: {
          ids: [(resource as AuthorizedRegistryView).id],
        },
      };
    },
    useSuccessHandler: ({ resources }) => {
      const invalidate = useRegistryDeletionEffects(
        (Array.isArray(resources) ? resources : [resources]).map((resource) => resource.id),
      );
      const navigate = useNavigate();
      return async () => {
        await invalidate();
        navigate(`/registries`);
      };
    },
  })
  .build();
