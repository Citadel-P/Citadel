import { Rocket, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentStatus, DeploymentView } from '@/api/generated/api.types';
import { useTaskSheet } from '@/lib/atoms';

export const { info: DeploymentActions } = createActionsBuilder<DeploymentView>()
  .addAction({
    key: 'deployToggle',
    type: 'toggle',
    predicate: (r: DeploymentView) => r.status !== DeploymentStatus.Created,
    primary: {
      title: 'Deploy',
      icon: Rocket,
      confirm: true,
      useHandler: ({ resources }) => {
        const { open: openSheet } = useTaskSheet('Deployment');
        const selected = Array.isArray(resources) ? resources[0] : resources;

        return {
          canExecute: !!selected,
          run: () => openSheet({ kind: 'deploy', payload: { deploymentId: selected.id, name: selected.name } }),
        };
      },
    },
    secondary: {
      title: 'Redeploy',
      icon: Rocket,
      confirm: true,
      useHandler: ({ resources }) => {
        const { open: openSheet } = useTaskSheet('Deployment');
        const selected = Array.isArray(resources) ? resources[0] : resources;

        return {
          canExecute: !!selected,
          run: () =>
            openSheet({ kind: 'deploy', payload: { deploymentId: selected.id, name: selected.name, redeploy: true } }),
        };
      },
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
