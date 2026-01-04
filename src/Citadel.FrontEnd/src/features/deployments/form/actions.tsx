import { Pause, Rocket, StepForward, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentStatus, DeploymentView } from '@/api/generated/api.types';
import { useTaskSheet } from '@/lib/atoms';

const useVariables = (resources: DeploymentView | DeploymentView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

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
          run: () => openSheet({ kind: 'deploy', payload: { id: selected.id, name: selected.name } }),
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
          run: () => openSheet({ kind: 'deploy', payload: { id: selected.id, name: selected.name, recreate: true } }),
        };
      },
    },
  })
  .addAction({
    key: 'pauseToggle',
    type: 'toggle',
    primary: {
      title: 'Suspend',
      icon: Pause,
      mutateKey: 'suspendDeployment',
      useVariables,
      canExecute: (r) => {
        const can = (x: DeploymentView) => x.status === DeploymentStatus.Healthy;
        return Array.isArray(r) ? r.some(can) : can(r);
      },
    },
    secondary: {
      title: 'Resume',
      icon: StepForward,
      mutateKey: 'resumeDeployment',
      useVariables,
      canExecute: (r) => {
        const can = (x: DeploymentView) => x.status === DeploymentStatus.Pending;
        return Array.isArray(r) ? r.some(can) : can(r);
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
