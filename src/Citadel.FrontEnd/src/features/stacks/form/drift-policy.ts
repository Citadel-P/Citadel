import { StackDriftMode, StackDriftPolicy } from '@/api/generated/api.types';

export const getDriftModePreset = (mode: StackDriftMode): StackDriftPolicy => {
  const observesDrift = mode !== StackDriftMode.Disabled;
  const autoFixesSafeDrift = mode === StackDriftMode.AutoFix;

  return {
    mode,
    alertOnDrift: observesDrift,
    markDegraded: observesDrift,
    autoStartStoppedContainers: autoFixesSafeDrift,
    autoResumePausedContainers: autoFixesSafeDrift,
    removeExtraContainers: false,
  };
};
