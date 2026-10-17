import type { NetworkView, RuntimeIpamConfig } from '@/api/generated/api.types';

export function networkAddresses(network: NetworkView, field: 'subnet' | 'gateway' | 'ipRange'): string[] {
  return [
    ...new Set(
      (network.ipam?.config ?? []).flatMap((config: RuntimeIpamConfig) => (config[field] ? [config[field]!] : [])),
    ),
  ];
}

export function networkProperties(network: NetworkView): string[] {
  return [
    network.internal && 'Internal',
    network.attachable && 'Attachable',
    network.ingress && 'Ingress',
    network.configOnly && 'Configuration only',
  ].filter((value): value is string => !!value);
}

// The list can report usage without returning every endpoint. An empty map
// therefore cannot establish that a network is unused or safe to remove.
export function networkUsageLabel(network: NetworkView): string {
  if (network.isStale) return 'Unknown';
  const count = Object.keys(network.containers ?? {}).length;
  if (count) return `${count} connected ${count === 1 ? 'container' : 'containers'}`;
  const services = new Set(
    network.usage?.entries
      .filter((entry) => entry.relationship === 'ServiceNetwork')
      .map((entry) => entry.dockerServiceId),
  );
  if (services.size) return `${services.size} observed ${services.size === 1 ? 'service' : 'services'}`;
  const references = new Set(
    network.usage?.entries
      .filter((entry) => entry.relationship === 'SavedReference')
      .map((entry) => `${entry.workloadKind}:${entry.workloadId}`),
  );
  if (references.size) return `${references.size} saved ${references.size === 1 ? 'reference' : 'references'}`;
  return network.inUse ? 'In use' : 'No connected containers reported';
}

export function filterNetworks(networks: NetworkView[], search: string): NetworkView[] {
  const term = search.trim().toLowerCase();
  if (!term) return networks;
  return networks.filter((network) =>
    [
      network.name,
      network.id,
      network.driver,
      network.scope,
      network.nodeHostname,
      ...networkAddresses(network, 'subnet'),
      ...(network.usage?.entries.flatMap((entry) => [entry.workloadName, entry.containerName, entry.serviceName]) ??
        []),
    ].some((value) => value?.toLowerCase().includes(term)),
  );
}

export function networkRowId(network: NetworkView): string {
  return JSON.stringify([network.dockerNodeId ?? null, network.id]);
}

// Shared by the list's map action and the map picker. Preserve filters while
// replacing the entire focus identity, including removal of an old node scope.
export function networkMapParams(current: URLSearchParams, platformId: string, network: NetworkView): URLSearchParams {
  const next = new URLSearchParams(current);
  next.delete('view');
  next.delete('networkContainer');
  next.set('network', network.id);
  next.set('networkPlatform', platformId);
  if (network.dockerNodeId) next.set('dockerNodeId', network.dockerNodeId);
  else next.delete('dockerNodeId');
  return next;
}

export function networkDetailsPath(platformId: string, network: NetworkView): string {
  const nodeQuery = network.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(network.dockerNodeId)}` : '';
  return `/platforms/${platformId}/networks/${encodeURIComponent(network.id)}/${nodeQuery}`;
}

// Usage visibility is actor-scoped. Never use a filtered attachment count alone
// to decide whether deletion is allowed; the server repeats the live checks.
export function canDeleteNetwork(network: NetworkView): boolean {
  return (
    !network.isSystem &&
    !network.isStale &&
    !network.dockerNodeId &&
    network.inUse === false &&
    !network.usage?.entries.some(
      (entry) => entry.relationship === 'SavedReference' || entry.relationship === 'ServiceNetwork',
    ) &&
    Object.keys(network.containers ?? {}).length === 0 &&
    !network.labels?.['com.docker.stack.namespace'] &&
    !network.labels?.['com.citadel.stack-id']
  );
}
