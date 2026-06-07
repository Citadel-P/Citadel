import { Eye, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { StackReleaseStatus, StackView, ResourceControlState } from '@/api/generated/api.types';
import { Ban, Pause, Play, Rocket, StepForward } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ActionConfig } from '@/components/custom/actions-builder';

export const useVariables = (resources: StackView | StackView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

export const isProcessing = (resource: StackView) => resource.controlState === ResourceControlState.Processing;

export const deployAction: ActionConfig<StackView, any> = {
  key: 'deployToggle',
  type: 'toggle',
  predicate: (r: StackView) => r.status !== StackReleaseStatus.Created,
  primary: {
    title: 'Deploy',
    icon: Rocket,
    confirm: true,
    resourceType: 'Stack',
    requiredCapabilities: ['canRead', 'canApply'],
    useHandler: ({ resources }) => {
      const { open: openSheet } = useTaskSheet('Stack');
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute:
          !!selected && selected.status !== StackReleaseStatus.Applying && !isProcessing(selected) && !multiSelect,
        run: () => openSheet({ kind: 'stack', payload: { id: selected.id, name: selected.name } }),
      };
    },
  },
  secondary: {
    title: 'Redeploy',
    icon: Rocket,
    confirm: true,
    resourceType: 'Stack',
    requiredCapabilities: ['canRead', 'canApply'],
    useHandler: ({ resources }) => {
      const { open: openSheet } = useTaskSheet('Stack');
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute:
          !!selected && selected.status !== StackReleaseStatus.Applying && !isProcessing(selected) && !multiSelect,
        run: () => openSheet({ kind: 'stack', payload: { id: selected.id, name: selected.name, recreate: true } }),
      };
    },
  },
};

export const startAction: ActionConfig<StackView, 'startDeployments'> = {
  key: 'start',
  type: 'command',
  icon: Play,
  mutateKey: 'startDeployments',
  useVariables,
  canExecute: (r) => {
    const can = (x: StackView) => x.status === StackReleaseStatus.Stopped && !isProcessing(x);
    return Array.isArray(r) ? r.every(can) : can(r);
  },
};

export const stopAction: ActionConfig<StackView, 'stopDeployments'> = {
  key: 'stop',
  type: 'command',
  icon: Ban,
  mutateKey: 'stopDeployments',
  useVariables,
  canExecute: (r) => {
    const can = (x: StackView) => x.status === StackReleaseStatus.Healthy && !isProcessing(x);
    return Array.isArray(r) ? r.every(can) : can(r);
  },
};

export const pauseAction: ActionConfig<StackView, 'pauseDeployments' | 'resumeDeployments'> = {
  key: 'pauseToggle',
  type: 'toggle',
  primary: {
    title: 'Pause',
    icon: Pause,
    mutateKey: 'pauseDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: StackView) => x.status === StackReleaseStatus.Healthy && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  },
  secondary: {
    title: 'Resume',
    icon: StepForward,
    mutateKey: 'resumeDeployments',
    useVariables,
    canExecute: (r) => {
      const can = (x: StackView) => x.status === StackReleaseStatus.Pending && !isProcessing(x);
      return Array.isArray(r) ? r.every(can) : can(r);
    },
  },
};

export const { dropdown: StackDropdownActions, group: StackGroupActions } =
  createActionsBuilder<StackView>()
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
            navigate(`/stacks/edit/${selected.id}/`);
          },
        };
      },
    })
    .addAction({
      key: 'delete',
      type: 'command',
      icon: Trash,
      mutateKey: 'deleteStacks',
      canExecute: () => true,
      separatorBefore: true,
      confirm: true,
      destructive: true,
      resourceType: 'Stack',
      useVariables,
    })
    .build();
