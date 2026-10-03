import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { TeamView } from '@/api/generated/api.types';

export const { info: TeamActions } = createActionsBuilder<TeamView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteTeams',
    confirm: true,
    destructive: true,
    resourceType: 'Team',
    useVariables: (resources) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      return {
        ids: selected.map((x) => x.id!),
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/access/teams`);
      };
    },
  })
  .build();
