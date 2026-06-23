import { Eye, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { StackDriftMode, StackReleaseStatus, StackView, ResourceControlState } from '@/api/generated/api.types';
import { Ban, Pause, Play, RefreshCw, Rocket, StepForward } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ActionConfig } from '@/components/custom/actions-builder';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';

export const useVariables = (resources: StackView | StackView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

export const isProcessing = (resource: StackView) => resource.controlState === ResourceControlState.Processing;

const everyStack = (resources: StackView | StackView[], predicate: (resource: StackView) => boolean) =>
  Array.isArray(resources) ? resources.every(predicate) : predicate(resources);

const hasStatus = (resource: StackView, ...statuses: StackReleaseStatus[]) => statuses.includes(resource.status);

const canControl = (resource: StackView, ...statuses: StackReleaseStatus[]) =>
  hasStatus(resource, ...statuses) && !isProcessing(resource);

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

export const startAction: ActionConfig<StackView, 'startStacks'> = {
  key: 'start',
  type: 'command',
  icon: Play,
  mutateKey: 'startStacks',
  useVariables,
  canExecute: (r) => {
    return everyStack(r, (x) => canControl(x, StackReleaseStatus.Stopped));
  },
};

export const syncAction: ActionConfig<StackView, any> = {
  key: 'sync',
  type: 'command',
  icon: RefreshCw,
  requiredCapabilities: ['canWrite'],
  useHandler: ({ resources }) => {
    const queryClient = useQueryClient();
    const selected = Array.isArray(resources) ? resources[0] : resources;
    const multiSelect = Array.isArray(resources) && resources.length > 1;
    const queryEnabled =
      !!selected &&
      !multiSelect &&
      selected.status !== StackReleaseStatus.Created &&
      selected.driftPolicy?.mode !== StackDriftMode.Disabled;
    const { data, isLoading, isFetching, refetch } = useRead(
      'getStackDrift',
      { stackId: selected?.id ?? '' },
      { enabled: queryEnabled },
    );
    const { mutateAsync, isPending } = useMutate('reconcileStack', {
      onSuccess: () => {
        if (!selected) return;
        queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: selected.id }] });
        queryClient.invalidateQueries({ queryKey: ['getStackConfig', { stackId: selected.id }] });
        refetch();
      },
    });

    const canExecute =
      !!selected &&
      queryEnabled &&
      !isProcessing(selected) &&
      !isLoading &&
      data?.data?.hasDrift === true;

    return {
      canExecute,
      isPending: isPending || isFetching,
      run: async () => {
        if (!selected || !canExecute) return;
        await mutateAsync({ stackId: selected.id });
      },
    };
  },
};

export const stopAction: ActionConfig<StackView, 'stopStacks'> = {
  key: 'stop',
  type: 'command',
  icon: Ban,
  mutateKey: 'stopStacks',
  useVariables,
  canExecute: (r) => {
    return everyStack(r, (x) => canControl(x, StackReleaseStatus.Healthy, StackReleaseStatus.Paused));
  },
};

export const pauseAction: ActionConfig<StackView, 'pauseStacks' | 'resumeStacks'> = {
  key: 'pauseToggle',
  type: 'toggle',
  primary: {
    title: 'Pause',
    icon: Pause,
    mutateKey: 'pauseStacks',
    useVariables,
    canExecute: (r) => {
      return everyStack(r, (x) => canControl(x, StackReleaseStatus.Healthy));
    },
  },
  secondary: {
    title: 'Resume',
    icon: StepForward,
    mutateKey: 'resumeStacks',
    useVariables,
    canExecute: (r) => {
      return everyStack(r, (x) => canControl(x, StackReleaseStatus.Paused));
    },
  },
};

export const { dropdown: StackDropdownActions, group: StackGroupActions } =
  createActionsBuilder<StackView>()
    .addAction(deployAction)
    .addAction(syncAction)
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
