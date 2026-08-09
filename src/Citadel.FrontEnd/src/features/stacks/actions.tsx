import { Copy, Eye, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { Ban, Pause, Play, RefreshCw, Rocket, StepForward } from 'lucide-react';
import { useTaskSheet } from '@/lib/atoms';
import { ActionConfig } from '@/components/custom/actions-builder';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import {
  StackReleaseStatus,
  ResourceControlState,
  StackDrift,
  StackDriftMode,
  StackDriftPolicy,
  StackDriftReport,
  StackReconciliationResult,
  StackReconciliationStatus,
  StackSource,
  StackView,
  PlatformType,
} from '@/api/generated/api.types';
import {
  getStackUpdateCheckDisabledReason,
  getStackGitUpdateState,
  getStackImageUpdateCheckMessage,
  hasStackUpdateAvailable,
} from './update-status';

export const useVariables = (resources: StackView | StackView[]) =>
  Array.isArray(resources) ? resources.map((r) => r.id) : [resources.id];

export const isProcessing = (resource: StackView) => resource.controlState === ResourceControlState.Processing;

const everyStack = (resources: StackView | StackView[], predicate: (resource: StackView) => boolean) =>
  Array.isArray(resources) ? resources.every(predicate) : predicate(resources);

const hasStatus = (resource: StackView, ...statuses: StackReleaseStatus[]) => statuses.includes(resource.status);

const canControl = (resource: StackView, ...statuses: StackReleaseStatus[]) =>
  resource.platformType !== PlatformType.DockerSwarm && hasStatus(resource, ...statuses) && !isProcessing(resource);

const canCheckDrift = (resource: StackView) =>
  resource.platformType !== PlatformType.DockerSwarm &&
  (resource.status === StackReleaseStatus.Healthy || resource.status === StackReleaseStatus.Degraded);

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
          !!selected &&
          selected.status !== StackReleaseStatus.Applying &&
          !isProcessing(selected) &&
          !multiSelect,
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
          !!selected &&
          selected.status !== StackReleaseStatus.Applying &&
          !isProcessing(selected) &&
          !multiSelect,
        run: () =>
          openSheet({
            kind: 'stack',
            payload: {
              id: selected.id,
              name: selected.name,
              recreate: selected.platformType !== PlatformType.DockerSwarm,
            },
          }),
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
  title: 'Reconcile drift',
  type: 'command',
  icon: RefreshCw,
  requiredCapabilities: ['canWrite'],
  useHandler: ({ resources }) => {
    const queryClient = useQueryClient();
    const selected = Array.isArray(resources) ? resources[0] : resources;
    const multiSelect = Array.isArray(resources) && resources.length > 1;
    const canCheck = !!selected && !multiSelect && canCheckDrift(selected);
    const { isFetching, refetch } = useRead('getStackDrift', { stackId: selected?.id ?? '' }, { enabled: false });
    const { mutateAsync, isPending } = useMutate('reconcileStack');

    const canExecute = !!selected && canCheck && !isProcessing(selected) && !isFetching;

    return {
      canExecute,
      isPending: isPending || isFetching,
      run: async () => {
        if (!selected || multiSelect || !canCheck || isProcessing(selected) || isFetching) return;

        const driftResult = await refetch();
        if (driftResult.error) {
          toast.error('Failed to check stack drift', { description: driftResult.error.message });
          return;
        }

        if (!hasActionableStackDrift(selected, driftResult.data?.data)) {
          toast.info('No actionable drift found');
          return;
        }

        const result = await mutateAsync({ stackId: selected.id });
        const message = getStackReconciliationToast(result.data);
        toast[message.kind](message.title, { description: message.description });
        queryClient.invalidateQueries({ queryKey: ['getStack', { stackId: selected.id }] });
        queryClient.invalidateQueries({ queryKey: ['getStackConfig', { stackId: selected.id }] });
        queryClient.invalidateQueries({ queryKey: ['getStackDrift', { stackId: selected.id }] });
      },
    };
  },
};

export const checkUpdatesAction: ActionConfig<StackView, any> = {
  key: 'checkUpdates',
  title: 'Check for updates',
  type: 'command',
  icon: RefreshCw,
  requiredCapabilities: ['canWrite'],
  useHandler: ({ resources }) => {
    const queryClient = useQueryClient();
    const selected = Array.isArray(resources) ? resources[0] : resources;
    const multiSelect = Array.isArray(resources) && resources.length > 1;
    const { mutateAsync, isPending } = useMutate('checkStackUpdates');
    const disabledReason = multiSelect
      ? 'Select one stack to check for updates.'
      : getStackUpdateCheckDisabledReason(selected);
    const canExecute = disabledReason === undefined;

    return {
      canExecute,
      disabledReason,
      isPending,
      run: async () => {
        if (!selected || !canExecute) return;

        try {
          const result = await mutateAsync({ stackId: selected.id });
          const updated = result.data;

          if (updated.stackSource === StackSource.Git) {
            if (hasStackUpdateAvailable(updated) && getStackGitUpdateState(updated)?.remoteCommitSha) {
              toast.info('Git update available', {
                description: 'A newer relevant commit is available.',
              });
            } else {
              toast.success('Stack source is up to date', {
                description: 'No newer relevant commit was found.',
              });
            }
          } else {
            const message = getStackImageUpdateCheckMessage(updated);
            toast[message.kind](message.title, { description: message.description });
          }

          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['getStack'] }),
            queryClient.invalidateQueries({ queryKey: ['listStacks'] }),
          ]);
        } catch {
          // Nope
        }
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

export const duplicateAction: ActionConfig<StackView, any> = {
  key: 'duplicate',
  type: 'command',
  icon: Copy,
  requiredCapabilities: ['canRead', 'canWrite'],
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
        navigate(`/stacks/add?duplicateFrom=${selected.id}`);
      },
    };
  },
};

export const { dropdown: StackDropdownActions, group: StackGroupActions } = createActionsBuilder<StackView>()
  .addAction(deployAction)
  .addAction(syncAction)
  .addAction(startAction)
  .addAction(stopAction)
  .addAction(pauseAction)
  .addAction(duplicateAction)
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

const canApplyStackDrift = (policy: StackDriftPolicy | null | undefined, drift: StackDrift): boolean => {
  if (!policy || policy.mode !== StackDriftMode.AutoFix) return false;

  switch (drift.$type) {
    case 'ContainerStopped':
      return policy.autoStartStoppedContainers;
    case 'ContainerPaused':
      return policy.autoResumePausedContainers;
    case 'ExtraContainer':
      return policy.removeExtraContainers;
    default:
      return false;
  }
};

export const hasActionableStackDrift = (
  stack: StackView | null | undefined,
  report: StackDriftReport | null | undefined,
): boolean =>
  !!stack &&
  !!report &&
  report.hasDrift &&
  !report.hasStructuralDrift &&
  report.drifts.some((drift) => canApplyStackDrift(stack.driftPolicy, drift));

export const getStackReconciliationToast = (
  result: StackReconciliationResult,
): { kind: 'success' | 'warning' | 'info' | 'error'; title: string; description?: string } => {
  if (result.status === StackReconciliationStatus.Reconciled) {
    return {
      kind: 'success',
      title: 'Stack drift reconciled',
    };
  }

  if (result.status === StackReconciliationStatus.NoDrift) {
    return {
      kind: 'info',
      title: 'No drift detected',
    };
  }

  if (result.status === StackReconciliationStatus.RequiresReapply) {
    return {
      kind: 'warning',
      title: 'Reapply required',
      description: 'This drift changes stack structure and cannot be safely reconciled.',
    };
  }

  if (result.actions.length === 0 && result.beforeReport.hasAutoFixableDrift) {
    return {
      kind: 'warning',
      title: 'No safe auto-fix action is enabled',
      description: 'Enable the matching safe auto-fix option in Config, then sync again.',
    };
  }

  if (result.status === StackReconciliationStatus.Failed) {
    return {
      kind: 'error',
      title: 'Stack drift reconciliation failed',
    };
  }

  return {
    kind: 'warning',
    title: 'Stack drift partially reconciled',
  };
};
