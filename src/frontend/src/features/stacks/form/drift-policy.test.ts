import { StackDriftMode } from '@/api/generated/api.types';
import { getDriftModePreset } from './drift-policy';

describe('getDriftModePreset', () => {
  it('enables observation without remediation for detect-only mode', () => {
    expect(getDriftModePreset(StackDriftMode.DetectOnly)).toEqual({
      mode: StackDriftMode.DetectOnly,
      alertOnDrift: true,
      markDegraded: true,
      autoStartStoppedContainers: false,
      autoResumePausedContainers: false,
      removeExtraContainers: false,
    });
  });

  it('enables safe remediation and keeps destructive cleanup off for auto-fix mode', () => {
    expect(getDriftModePreset(StackDriftMode.AutoFix)).toEqual({
      mode: StackDriftMode.AutoFix,
      alertOnDrift: true,
      markDegraded: true,
      autoStartStoppedContainers: true,
      autoResumePausedContainers: true,
      removeExtraContainers: false,
    });
  });

  it('clears drift behavior when disabled', () => {
    expect(getDriftModePreset(StackDriftMode.Disabled)).toEqual({
      mode: StackDriftMode.Disabled,
      alertOnDrift: false,
      markDegraded: false,
      autoStartStoppedContainers: false,
      autoResumePausedContainers: false,
      removeExtraContainers: false,
    });
  });
});
