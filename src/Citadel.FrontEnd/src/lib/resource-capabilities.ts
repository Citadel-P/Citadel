export type CapabilityKey =
  | 'canRead'
  | 'canWrite'
  | 'canExecute'
  | 'canApply'
  | 'canInspect'
  | 'canViewLogs'
  | 'canOpenTerminal'
  | 'canPull'
  | 'canViewResourceBindings'
  | 'canViewReleases';

type ResourceWithCapabilities = {
  capabilities?: null | Partial<Record<CapabilityKey, boolean>>;
};

const IMPLIED_CAPABILITIES: Partial<Record<CapabilityKey, CapabilityKey[]>> = {
  canRead: ['canWrite', 'canExecute'],
  canWrite: ['canExecute'],
};

export const hasCapability = (resource: unknown, key: CapabilityKey): boolean => {
  
  const capabilities = (resource as ResourceWithCapabilities | null | undefined)?.capabilities;

  if (capabilities == null) return true;

  if (capabilities[key] === true) return true;

  return IMPLIED_CAPABILITIES[key]?.some((impliedBy) => capabilities[impliedBy] === true) ?? false;
};

export const hasCapabilities = (resources: unknown | unknown[], keys: CapabilityKey[] = []): boolean => {
  if (keys.length === 0) return true;

  const items = Array.isArray(resources) ? resources : [resources];

  return items.every((resource) => keys.every((key) => hasCapability(resource, key)));
};
