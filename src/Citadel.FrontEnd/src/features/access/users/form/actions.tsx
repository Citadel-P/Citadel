import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { UserView } from '@/api/generated/api.types';

export const { info: UserActions } = createActionsBuilder<UserView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteUsers',
    confirm: true,
    destructive: true,
    resourceType: 'User',
    useVariables: (resources) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        ids: selected.map((x) => x.id!),
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/access/users`);
      };
    },
  })
  .build();
