import {
  AutoUpdateStatus,
  ImageUpdateState,
  RecreateStackOnNewCommitState,
  StackView,
} from '@/api/generated/api.types';

export const getStackImageUpdateStates = (stack: StackView): ImageUpdateState[] =>
  stack.stackUpdateState?.recreateStackOnNewImageState?.autoUpdateStates ?? [];

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
