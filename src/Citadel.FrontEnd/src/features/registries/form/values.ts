import type { RegistryConfigurationView, RegistryConfigurationPatch, RegistryPatch } from '@/api/generated/api.types';

/** Presence flags are display metadata, never part of a configuration update. */
export function registrySettings(configuration: RegistryConfigurationView): RegistryConfigurationPatch {
  return Object.fromEntries(
    Object.entries(configuration).filter(([key]) => !['hasPat', 'hasPassword', 'hasSecretAccessKey'].includes(key)),
  );
}

/** Empty replacement fields keep the stored credential. API null remains an explicit clear. */
export function registryUpdate(value: RegistryPatch): RegistryPatch {
  if (!value.configuration) return value;
  const configuration = { ...value.configuration };
  for (const key of ['pat', 'password', 'secretAccessKey'] as const) {
    if (configuration[key] === '') delete configuration[key];
  }
  return { ...value, configuration };
}
