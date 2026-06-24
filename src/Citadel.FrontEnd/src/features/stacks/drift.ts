import {
  StackDrift,
  StackDriftMode,
  StackDriftPolicy,
  StackDriftReport,
  StackReconciliationResult,
  StackReconciliationStatus,
  StackView,
} from '@/api/generated/api.types';

export const canApplyStackDrift = (policy: StackDriftPolicy | null | undefined, drift: StackDrift): boolean => {
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
