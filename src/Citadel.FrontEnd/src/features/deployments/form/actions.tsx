import { Rocket, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentView } from '@/api/generated/api.types';
import { useTaskSheet } from '@/lib/atoms';

export const { info: DeploymentActions } = createActionsBuilder<DeploymentView>()
  .addAction({
    key: 'deploy',
    type: 'command',
    icon: Rocket,
    useHandler: ({ resources }) => {
      const { open: openSheet } = useTaskSheet('Deployment');
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const canExecute = !!selected;

      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !selected) return;
          openSheet({
            kind: 'deploy',
            payload: {
              deploymentId: selected.id,
              name: selected.name,
            },
          });
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteDeployments',
    confirm: true,
    destructive: true,
    resourceType: 'Deployment',
    canExecute: () => true,
    useVariables: (resource) => {
      return {
        ids: [(resource as DeploymentView).id],
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/deployments`);
      };
    },
  })
  .build();
