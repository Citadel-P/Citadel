import {
  AutoUpdateStatus,
  ImageUpdateState,
  RecreateStackOnNewCommitState,
  ResourceControlState,
  StackReleaseStatus,
  StackSource,
  StackView,
} from '@/api/generated/api.types';

export const canCheckStackUpdates = (stack: StackView | null | undefined): boolean =>
  getStackUpdateCheckDisabledReason(stack) === undefined;

export const getStackUpdateCheckDisabledReason = (stack: StackView | null | undefined): string | undefined => {
  if (!stack) return 'Select a stack to check for updates.';
  if (stack.status === StackReleaseStatus.Created) {
    return 'Apply this stack before checking for updates.';
  }
  if (stack.controlState === ResourceControlState.Processing) {
    return 'Wait for the current stack operation to finish.';
  }

  const allowedStatuses = [
    StackReleaseStatus.Healthy,
    StackReleaseStatus.Degraded,
    StackReleaseStatus.Stopped,
    StackReleaseStatus.Paused,
  ];
  if (!allowedStatuses.includes(stack.status)) {
    return 'The current stack state does not support update checks.';
  }

  if (stack.stackSource === StackSource.Git) {
    const spec = stack.spec?.$type === 'Git' ? stack.spec : null;
    const source = stack.source;
    const hasMatchingAppliedCommit =
      !!spec &&
      source?.sourceType === StackSource.Git &&
      source.gitRepositoryId === spec.gitRepoId &&
      source.branch === spec.branch &&
      !!source.resolvedCommitSha?.trim();

    if (!hasMatchingAppliedCommit) {
      return 'Redeploy this stack once so Citadel has an applied Git commit to compare.';
    }
  }

  return undefined;
};

export const getStackImageUpdateStates = (stack: StackView): ImageUpdateState[] =>
  stack.stackUpdateState?.recreateStackOnNewImageState?.autoUpdateStates ?? [];

export type StackImageUpdateCheckMessage = {
  kind: 'info' | 'success';
  title: string;
  description: string;
};

export const getStackImageUpdateCheckMessage = (stack: StackView): StackImageUpdateCheckMessage => {
  const states = getStackImageUpdateStates(stack);
  const updateCount = states.filter((state) => state.updateAvailable).length;

  if (updateCount > 0) {
    return {
      kind: 'info',
      title: 'Updates available',
      description: `${
        updateCount === 1 ? '1 service image has a newer digest.' : `${updateCount} service images have newer digests.`
      } Redeploy the stack to apply the ${updateCount === 1 ? 'change' : 'changes'}.`,
    };
  }

  return {
    kind: 'success',
    title: 'Stack images are up to date',
    description: 'No newer service image digest was found.',
  };
};

export const getStackGitUpdateState = (stack: StackView): RecreateStackOnNewCommitState | null => {
  const state = stack.stackUpdateState;
  if (!state || !('recreateStackOnNewCommitState' in state)) return null;

  const gitState = state.recreateStackOnNewCommitState;
  return gitState?.currentCommitSha ? gitState : null;
};

export const getStackUpdateStatus = (stack: StackView): AutoUpdateStatus => {
  const gitState = getStackGitUpdateState(stack);
  if (gitState?.remoteCommitSha && gitState.remoteCommitSha !== gitState.currentCommitSha) {
    return AutoUpdateStatus.UpdateAvailable;
  }

  if (gitState?.currentCommitSha) return AutoUpdateStatus.UpToDate;

  const states = getStackImageUpdateStates(stack);
  if (states.length === 0) return AutoUpdateStatus.Unknown;
  if (states.some((state) => state.updateAvailable)) return AutoUpdateStatus.UpdateAvailable;
  return AutoUpdateStatus.UpToDate;
};

export const hasStackUpdateAvailable = (stack: StackView | null | undefined): boolean =>
  !!stack && getStackUpdateStatus(stack) === AutoUpdateStatus.UpdateAvailable;

export const getLatestStackUpdateCheckTime = (stack: StackView): number | undefined => {
  const candidates = [
    getStackGitUpdateState(stack)?.lastCheckedAt,
    ...getStackImageUpdateStates(stack).map((state) => state.lastCheckedAt),
  ];

  return candidates
    .map((value) => (value ? new Date(value).getTime() : Number.NaN))
    .filter((value) => !Number.isNaN(value))
    .sort((a, b) => b - a)[0];
};
