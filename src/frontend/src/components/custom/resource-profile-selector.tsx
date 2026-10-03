import { ItemSelector } from '@/components/custom/form-builder';

export const resourceProfiles = {
  automatic: {
    label: 'Automatic',
    description: 'Resources will be allocated automatically by the platform',
    cpuCores: null,
    memoryMiB: null,
  },
  xsmall: {
    label: 'X-Small',
    description: '0.25 CPU - 256 MB RAM',
    cpuCores: 0.25,
    memoryMiB: 256,
  },
  small: {
    label: 'Small',
    description: '0.5 CPU - 512 MB RAM',
    cpuCores: 0.5,
    memoryMiB: 512,
  },
  medium: {
    label: 'Medium',
    description: '0.5 CPU - 1 GB RAM',
    cpuCores: 0.5,
    memoryMiB: 1024,
  },
  large: {
    label: 'Large',
    description: '1.0 CPU - 2 GB RAM',
    cpuCores: 1,
    memoryMiB: 2048,
  },
  xlarge: {
    label: 'X-Large',
    description: '2.0 CPU - 4 GB RAM',
    cpuCores: 2,
    memoryMiB: 4096,
  },
} as const;

export type ResourceProfile = keyof typeof resourceProfiles;

export const findResourceProfile = (
  cpuCores: number | null | undefined,
  memoryMiB: number | null | undefined,
): ResourceProfile => {
  if (cpuCores == null && memoryMiB == null) return 'automatic';

  const match = Object.entries(resourceProfiles).find(
    ([key, profile]) => key !== 'automatic' && profile.cpuCores === cpuCores && profile.memoryMiB === memoryMiB,
  );
  return (match?.[0] as ResourceProfile | undefined) ?? 'automatic';
};

export function ResourceProfileSelector({
  value,
  onChange,
  disabled,
}: {
  value: ResourceProfile;
  onChange: (profile: ResourceProfile) => void;
  disabled?: boolean;
}) {
  return (
    <ItemSelector
      collection={resourceProfiles}
      value={value}
      disabled={disabled}
      onChange={onChange}
    />
  );
}
