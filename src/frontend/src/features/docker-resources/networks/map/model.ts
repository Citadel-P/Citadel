import { NetworkRelationship, type NetworkUsageEntry, type NetworkView } from '@/api/generated/api.types';
import { networkAddresses } from '../network-presentation';

export const MAX_MAP_ENTRIES = 50;
export type MapNode = {
  id: string;
  parentId?: string;
  kind: 'network' | 'container' | 'stack' | 'service' | 'reference';
  label: string;
  subtitle: string;
  address?: string;
  inUse?: boolean;
  isStale?: boolean;
  expanded?: boolean;
  entries: NetworkUsageEntry[];
  network?: NetworkView;
  searchTerms?: string[];
};
export type MapEdge = { id: string; source: string; target: string; saved: boolean; service?: boolean; label?: string };
export type NetworkGraph = { nodes: MapNode[]; edges: MapEdge[]; truncated: boolean };
export type NetworkOwner = { id: string; kind: 'Deployment' | 'Stack' | 'SwarmService' };

const entryIdentity = (entry: NetworkUsageEntry) =>
  JSON.stringify([entry.relationship, entry.containerId, entry.dockerNodeId, entry.dockerServiceId, entry.reference]);

/** Scope existing authorized observations by persisted owner identity, never by name. */
export function ownerGraph(
  platformId: string,
  owner: NetworkOwner,
  networks: NetworkView[],
  showReferences: boolean,
  expanded: ReadonlySet<string>,
): NetworkGraph {
  const nodes = new Map<string, MapNode>();
  const edges = new Map<string, MapEdge>();
  let remaining = MAX_MAP_ENTRIES;
  let truncated = networks.some((network) => !network.usage || network.usage.truncated);
  const ordered = [...networks].sort((a, b) =>
    JSON.stringify([a.dockerNodeId ?? null, a.id]).localeCompare(JSON.stringify([b.dockerNodeId ?? null, b.id])),
  );
  for (const network of ordered) {
    const owned =
      network.usage?.entries.filter((entry) => entry.workloadKind === owner.kind && entry.workloadId === owner.id) ??
      [];
    // Prefer observed membership on this network; a saved reference must not
    // duplicate the same workload while it is connected.
    const observed = owned.filter((entry) => entry.relationship !== 'SavedReference');
    const matching = observed.length || !showReferences ? observed : owned;
    if (matching.length > remaining) truncated = true;
    const entries = matching.slice(0, remaining);
    remaining -= entries.length;
    if (!entries.length) continue;
    const scoped = {
      ...network,
      containers: Object.fromEntries(
        Object.entries(network.containers ?? {}).filter(([id]) =>
          entries.some((entry) => entry.relationship === 'Attachment' && entry.dockerContainerId === id),
        ),
      ),
      usage: { ...network.usage!, entries },
    };
    const graph = networkGraph(platformId, scoped, showReferences, expanded);
    for (const node of graph.nodes) {
      const previous = nodes.get(node.id);
      if (!previous) nodes.set(node.id, node);
      else {
        const distinct = new Map([...previous.entries, ...node.entries].map((entry) => [entryIdentity(entry), entry]));
        previous.entries = [...distinct.values()];
        previous.isStale ||= node.isStale;
        previous.address = [
          ...new Set([previous.address, node.address].flatMap((value) => value?.split(' · ') ?? [])),
        ].join(' · ');
        if (previous.network && node.network) {
          previous.inUse = previous.inUse || node.inUse;
          previous.network = {
            ...previous.network,
            // A merged overlay is cluster-scoped, not one worker's local network.
            dockerNodeId: null,
            nodeHostname: null,
            inUse: !!previous.inUse,
            isStale: previous.isStale ?? false,
            containers: { ...previous.network.containers, ...node.network.containers },
            usage: {
              isPartial: true,
              truncated: !!previous.network.usage?.truncated || !!node.network.usage?.truncated,
              entries: [
                ...new Map(
                  [...previous.network.usage!.entries, ...node.network.usage!.entries].map((entry) => [
                    entryIdentity(entry),
                    entry,
                  ]),
                ).values(),
              ],
            },
          };
          // A logical overlay can combine node-local observations. If a Docker
          // ID is ambiguous across nodes, don't attribute one node's address to both.
          const endpointNodes = new Map<string, Set<string | null>>();
          for (const entry of previous.network.usage!.entries) {
            if (!entry.dockerContainerId || entry.relationship !== 'Attachment') continue;
            const scopes = endpointNodes.get(entry.dockerContainerId) ?? new Set();
            scopes.add(entry.dockerNodeId ?? null);
            endpointNodes.set(entry.dockerContainerId, scopes);
            if (scopes.size > 1) delete previous.network.containers[entry.dockerContainerId];
          }
        }
      }
    }
    for (const edge of graph.edges) edges.set(edge.id, edge);
  }
  for (const node of nodes.values()) {
    if (node.kind !== 'stack' && node.kind !== 'service') continue;
    const containers = node.entries.filter((entry) => entry.relationship === 'Attachment').length;
    const services = node.entries.filter((entry) => entry.relationship === 'ServiceNetwork').length;
    node.subtitle = [
      containers && `${containers} connected ${containers === 1 ? 'container' : 'containers'}`,
      services && `${services} service ${services === 1 ? 'network' : 'networks'}`,
    ]
      .filter(Boolean)
      .join(' · ');
  }
  return { nodes: [...nodes.values()], edges: [...edges.values()], truncated };
}

export function networkGraph(
  platformId: string,
  network: NetworkView,
  showReferences: boolean,
  expanded: ReadonlySet<string>,
): NetworkGraph {
  const identity = (kind: string, ...parts: (string | null | undefined)[]) =>
    JSON.stringify([platformId, kind, ...parts]);
  const root = identity('network', network.scope === 'swarm' ? null : network.dockerNodeId, network.id);
  const nodes: MapNode[] = [
    {
      id: root,
      kind: 'network',
      label: network.name,
      subtitle: `${network.driver} · ${network.scope}`,
      address: networkAddresses(network, 'subnet').join(' · '),
      inUse: network.inUse,
      entries: [],
      network,
    },
  ];
  const edges: MapEdge[] = [];
  const allEntries = network.usage?.entries ?? [];
  // Bound before grouping; never infer additional members from saved configuration.
  const entries = allEntries.slice(0, MAX_MAP_ENTRIES);
  const attachments = new Map<string, NetworkUsageEntry>();
  for (const entry of entries) {
    if (entry.relationship !== 'Attachment' || !entry.containerId || !entry.dockerContainerId) continue;
    attachments.set(identity('container', entry.dockerNodeId, entry.dockerContainerId), entry);
  }
  const groups = new Map<string, { node: MapNode; children: MapNode[] }>();
  const connect = (target: string, saved = false, service = false) =>
    edges.push({ id: identity('edge', root, target), source: root, target, saved, service });
  for (const [id, entry] of [...attachments].sort(([a], [b]) => a.localeCompare(b))) {
    const endpoint = network.containers?.[entry.dockerContainerId!];
    const node: MapNode = {
      id,
      kind: 'container',
      label: entry.containerName || 'Container',
      subtitle: entry.workloadName ? `${entry.workloadKind} · ${entry.workloadName}` : 'Container',
      address: [endpoint?.ipV4Address, endpoint?.ipv6Address].filter(Boolean).join(' · '),
      entries: [entry],
    };
    if (entry.workloadId && (entry.workloadKind === 'Stack' || entry.workloadKind === 'SwarmService')) {
      const groupId = identity(entry.workloadKind, entry.workloadId);
      let group = groups.get(groupId);
      if (!group) {
        group = {
          node: {
            id: groupId,
            kind: entry.workloadKind === 'Stack' ? 'stack' : 'service',
            label: entry.workloadName || entry.workloadKind,
            subtitle: '',
            entries: [],
            expanded: entry.workloadKind === 'Stack' && expanded.has(groupId),
          },
          children: [],
        };
        groups.set(groupId, group);
      }
      group.children.push(node);
      group.node.entries.push(entry);
    } else {
      nodes.push(node);
      connect(id);
    }
  }
  for (const { node, children } of [...groups.values()].sort((a, b) => a.node.id.localeCompare(b.node.id))) {
    node.subtitle = `${children.length} connected ${children.length === 1 ? 'container' : 'containers'}`;
    nodes.push(node);
    if (node.expanded) {
      for (const child of children) {
        nodes.push({ ...child, parentId: node.id });
        connect(child.id);
      }
    } else connect(node.id);
  }
  for (const entry of entries.filter((value) => value.relationship === 'ServiceNetwork' && value.dockerServiceId)) {
    const id =
      entry.workloadKind === 'SwarmService' && entry.workloadId
        ? identity('SwarmService', entry.workloadId)
        : identity('service', entry.dockerServiceId);
    const existing = nodes.find((node) => node.id === id);
    if (existing) {
      existing.entries.push(entry);
      existing.subtitle += ' · Service network';
      continue;
    }
    const service: MapNode = {
      id,
      kind: 'service',
      label: entry.serviceName || 'Swarm service',
      subtitle: entry.isStale ? 'Service observation out of date' : 'Observed service network',
      entries: [entry],
    };
    if (entry.workloadKind === 'Stack' && entry.workloadId) {
      const groupId = identity('Stack', entry.workloadId);
      let group = nodes.find((node) => node.id === groupId);
      if (!group) {
        group = {
          id: groupId,
          kind: 'stack',
          label: entry.workloadName || 'Stack',
          subtitle: '',
          entries: [],
          expanded: expanded.has(groupId),
        };
        nodes.push(group);
        if (!group.expanded) connect(groupId, false, true);
      }
      group.entries.push(entry);
      const tasks = group.entries.filter((value) => value.relationship === 'Attachment').length;
      const services = group.entries.filter((value) => value.relationship === 'ServiceNetwork').length;
      group.subtitle = `${services} service ${services === 1 ? 'network' : 'networks'} · ${tasks} task attachments`;
      if (group.expanded) {
        nodes.push({ ...service, parentId: groupId });
        connect(id, false, true);
      }
    } else {
      nodes.push(service);
      connect(id, false, true);
    }
  }
  if (showReferences) {
    const refs = new Map<string, NetworkUsageEntry[]>();
    for (const entry of entries) {
      if (entry.relationship !== 'SavedReference' || !entry.workloadId || !entry.workloadKind) continue;
      const id = identity('reference', entry.workloadKind, entry.workloadId);
      refs.set(id, [...(refs.get(id) ?? []), entry]);
    }
    for (const [id, values] of [...refs].sort(([a], [b]) => a.localeCompare(b))) {
      nodes.push({
        id,
        kind: 'reference',
        label: values[0].workloadName || values[0].workloadKind!,
        subtitle: `${values[0].workloadKind} · Configured use`,
        entries: values,
      });
      connect(id, true);
    }
  }
  return {
    nodes: nodes.map((node) => ({ ...node, isStale: network.isStale })),
    edges,
    truncated: !!network.usage?.truncated || allEntries.length > MAX_MAP_ENTRIES,
  };
}

// Only topology and fixed dimensions influence layout, not names, addresses or usage updates.
export function layoutStructure(graph: NetworkGraph) {
  return JSON.stringify({
    nodes: graph.nodes.map(({ id, parentId, expanded }) => ({ id, parentId, expanded: !!expanded })),
    edges: graph.edges.map(({ id, source, target }) => ({ id, source, target })),
  });
}

export function workloadGraph(
  platformId: string,
  container: import('@/api/generated/api.types').NetworkContainer,
  networking: import('@/api/generated/api.types').ContainerNetworkingView,
  inventory: NetworkView[],
): NetworkGraph {
  const id = JSON.stringify([platformId, 'container', container.dockerNodeId, container.dockerContainerId]);
  const entry: NetworkUsageEntry = { ...container, relationship: NetworkRelationship.Attachment };
  const member: MapNode = {
    id,
    kind: 'container',
    label: container.containerName,
    subtitle: container.workloadName ? `${container.workloadKind} · ${container.workloadName}` : 'Container',
    entries: [
      {
        ...container,
        relationship: networking.endpoints.some((endpoint) => endpoint.attached === true)
          ? NetworkRelationship.Attachment
          : NetworkRelationship.SavedReference,
      },
    ],
    isStale: container.isStale,
    searchTerms: networking.endpoints.flatMap((endpoint) => [
      ...endpoint.aliases,
      ...(endpoint.attached === true ? endpoint.addresses : []),
    ]),
  };
  const nodes: MapNode[] = [member];
  const edges: MapEdge[] = [];
  if (networking.mode === 'networks') {
    for (const endpoint of networking.endpoints.slice(0, MAX_MAP_ENTRIES)) {
      // Never match a same-named network on another node, nor invent missing inventory.
      const network = inventory.find(
        (candidate) =>
          candidate.id === endpoint.networkId &&
          (candidate.scope === 'swarm' || (candidate.dockerNodeId ?? null) === (container.dockerNodeId ?? null)),
      );
      if (!network) continue;
      const attached = endpoint.attached === true;
      const containers = { ...network.containers };
      delete containers[container.dockerContainerId];
      if (attached) {
        containers[container.dockerContainerId] = {
          name: container.containerName,
          endpointId: null,
          macAddress: null,
          ipV4Address: endpoint.addresses.find((address) => !address.includes(':')) ?? null,
          ipv6Address: endpoint.addresses.find((address) => address.includes(':')) ?? null,
        };
      }
      const observedNetwork: NetworkView = {
        ...network,
        inUse: attached || network.inUse,
        containers,
        usage: {
          isPartial: true,
          truncated: network.usage?.truncated ?? false,
          entries: [
            ...(network.usage?.entries.filter(
              (value) => value.containerId !== container.containerId || value.relationship !== 'Attachment',
            ) ?? []),
            ...(attached ? [entry] : []),
          ],
        },
      };
      const node = networkGraph(platformId, observedNetwork, false, new Set()).nodes[0];
      if (nodes.some((value) => value.id === node.id)) continue;
      nodes.push(node);
      edges.push({
        id: JSON.stringify([node.id, id]),
        source: node.id,
        target: id,
        saved: !attached,
        label: attached
          ? undefined
          : endpoint.attached === false
            ? 'Configured · Not attached'
            : 'Attachment unconfirmed',
      });
    }
  }
  return { nodes, edges, truncated: networking.truncated };
}

export function highlightedNodes(graph: NetworkGraph, selected?: string): Set<string> {
  const related = new Set<string>();
  if (!selected) return related;
  related.add(selected);
  for (const node of graph.nodes) if (node.parentId === selected) related.add(node.id);
  const direct = new Set(related);
  for (const edge of graph.edges) {
    if (direct.has(edge.source)) related.add(edge.target);
    else if (edge.target === selected || graph.nodes.find((node) => node.id === edge.target)?.parentId === selected)
      related.add(edge.source);
  }
  return related;
}
