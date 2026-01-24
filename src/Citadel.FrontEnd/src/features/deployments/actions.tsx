import { Eye, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { DeploymentStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';
import { Ban, Pause, Play, Rocket, StepForward } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ActionConfig } from '@/components/custom/actions-builder';

export const useVariables = (resources: DeploymentView | DeploymentView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

export const isProcessing = (resource: DeploymentView) => resource.controlState === ResourceControlState.Processing;

export const deployAction: ActionConfig<DeploymentView, any> = {
  key: 'deployToggle',
  type: 'toggle',
  predicate: (r: DeploymentView) => r.status !== DeploymentStatus.Created,
  primary: {
    title: 'Deploy',
    icon: Rocket,
    confirm: true,
    resourceType: 'Deployment',
    useHandler: ({ resources }) => {
      const { open: openSheet } = useTaskSheet('Deployment');
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute:
          !!selected && selected.status !== DeploymentStatus.Applying && !isProcessing(selected) && !multiSelect,
        run: () => openSheet({ kind: 'deploy', payload: { id: selected.id, name: selected.name } }),
      };
    },
  },
  secondary: {
    title: 'Redeploy',
    icon: Rocket,
    confirm: true,
    resourceType: 'Deployment',
    useHandler: ({ resources }) => {
      const { open: openSheet } = useTaskSheet('Deployment');
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute:
          !!selected && selected.status !== DeploymentStatus.Applying && !isProcessing(selected) && !multiSelect,
        run: () => openSheet({ kind: 'deploy', payload: { id: selected.id, name: selected.name, recreate: true } }),
      };
    },
  },
};

export const startAction: ActionConfig<DeploymentView, 'startDeployments'> = {
  key: 'start',
  type: 'command',
  icon: Play,
  mutateKey: 'startDeployments',
  useVariables,
  canExecute: (r) => {
    const can = (x: DeploymentView) => x.status === DeploymentStatus.Stopped && !isProcessing(x);
    return Array.isArray(r) ? r.every(can) : can(r);
  },
};

export const stopAction: ActionConfig<DeploymentView, 'stopDeployments'> = {
  key: 'stop',
  type: 'command',
  icon: Ban,
  mutateKey: 'stopDeployments',
  useVariables,
  canExecute: (r) => {
    const can = (x: DeploymentView) => x.status === DeploymentStatus.Healthy && !isProcessing(x);
    return Array.isArray(r) ? r.every(can) : can(r);
  },
};

export const pauseAction: ActionConfig<DeploymentView, 'pauseDeployments' | 'resumeDeployments'> = {
  key: 'pauseToggle',
  type: 'toggle',
  primary: {
    title: 'Pause',
    icon: Pause,
    mutateKey: 'pauseDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: DeploymentView) => x.status === DeploymentStatus.Healthy && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  },
  secondary: {
    title: 'Resume',
    icon: StepForward,
    mutateKey: 'resumeDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: DeploymentView) => x.status === DeploymentStatus.Pending && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  },
};

export const { dropdown: DeploymentDropdownActions, group: DeploymentGroupActions } =
  createActionsBuilder<DeploymentView>()
    .addAction(deployAction)
    .addAction(startAction)
    .addAction(stopAction)
    .addAction(pauseAction)
    .addAction({
      key: 'details',
      type: 'command',
      separatorBefore: true,
      icon: Eye,
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const selected = Array.isArray(resources) ? resources[0] : resources;
        const multiSelect = Array.isArray(resources) && resources.length > 1;
        const canExecute = !!selected && !multiSelect;

        return {
          canExecute,
          isPending: false,
          run: () => {
            if (!canExecute || !selected) return;
            navigate(`/deployments/edit/${selected.id}/`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteDeployments',
      canExecute: () => true,
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Deployment',
      useVariables,
    })
    .build();
