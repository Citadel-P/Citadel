import {
  AutoUpdateStatus,
  ImageUpdateState,
  RecreateStackOnNewCommitState,
  ResourceControlState,
  StackReleaseStatus,
  StackView,
} from '@/api/generated/api.types';

export const canCheckStackUpdates = (stack: StackView | null | undefined): boolean =>
  !!stack && stack.status !== StackReleaseStatus.Created && stack.controlState !== ResourceControlState.Processing;

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
