import { Ban, Pause, Play, Rocket, StepForward, Trash } from 'lucide-react';
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
          canExecute: !!selected && selected.status !== DeploymentStatus.Applying,
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
          canExecute: !!selected && selected.status !== DeploymentStatus.Applying,
          run: () => openSheet({ kind: 'deploy', payload: { id: selected.id, name: selected.name, recreate: true } }),
        };
      },
    },
  })
  .addAction({
    key: 'start',
    type: 'command',
    icon: Play,
    mutateKey: 'startDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: DeploymentView) => x.status === DeploymentStatus.Stopped;
      return Array.isArray(r) ? r.some(can) : can(r);
    },
  })
  .addAction({
    key: 'stop',
    type: 'command',
    icon: Ban,
    mutateKey: 'stopDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: DeploymentView) => x.status === DeploymentStatus.Healthy;
      return Array.isArray(r) ? r.some(can) : can(r);
    },
  })
  .addAction({
    key: 'pauseToggle',
    type: 'toggle',
    primary: {
      title: 'Pause',
      icon: Pause,
      mutateKey: 'pauseDeployments',
      useVariables,
      canExecute: (r) => {
        const can = (x: DeploymentView) => x.status === DeploymentStatus.Healthy;
        return Array.isArray(r) ? r.some(can) : can(r);
      },
    },
    secondary: {
      title: 'Resume',
      icon: StepForward,
      mutateKey: 'resumeDeployments',
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
    useVariables,
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/deployments`);
      };
    },
  })
  .build();
