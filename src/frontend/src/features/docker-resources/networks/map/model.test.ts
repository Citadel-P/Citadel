import ELK from 'elkjs/lib/elk.bundled.js';
import {
  NetworkRelationship,
  NetworkWorkloadKind,
  type NetworkUsageEntry,
  type NetworkView,
} from '@/api/generated/api.types';
import { workloadGraph, layoutStructure, MAX_MAP_ENTRIES, networkGraph, ownerGraph } from './model';
import { elkGraph, layoutPositions } from './layout';

const attachment = (id: string, overrides: Partial<NetworkUsageEntry> = {}): NetworkUsageEntry => ({
  isStale: false,
  workloadProcessing: false,
  relationship: NetworkRelationship.Attachment,
  containerId: `uuid-${id}`,
  dockerContainerId: id,
  containerName: id,
  ...overrides,
});
const network = (entries: NetworkUsageEntry[], overrides: Partial<NetworkView> = {}): NetworkView =>
  ({
    id: 'proxy',
    name: 'proxy',
    scope: 'local',
    dockerNodeId: null,
    driver: 'bridge',
    inUse: true,
    containers: {},
    usage: { entries, isPartial: true, truncated: false },
    ...overrides,
  }) as NetworkView;
const stack = { workloadKind: NetworkWorkloadKind.Stack, workloadId: 'stack-uuid', workloadName: 'Application' };

it('scopes a workload graph by owner identity and merges members attached to multiple networks', () => {
  const entries = [
    attachment('app', stack),
    attachment('db', stack),
    attachment('unrelated', {
      ...stack,
      workloadId: 'other-stack',
    }),
  ];
  const first = network(entries, {
    containers: {
      unrelated: { name: 'unrelated', endpointId: null, macAddress: null, ipV4Address: '10.0.0.9', ipv6Address: null },
    },
  });
  const second = network(entries, { id: 'backend', name: 'backend' });
  const graph = ownerGraph('p', { kind: 'Stack', id: 'stack-uuid' }, [first, second], false, new Set());
  expect(graph.nodes.filter((n) => n.kind === 'network')).toHaveLength(2);
  const group = graph.nodes.find((n) => n.kind === 'stack')!;
  expect(group.entries).toHaveLength(2);
  expect(group.subtitle).toBe('2 connected containers');
  expect(graph.edges).toHaveLength(2);
  expect(layoutStructure(ownerGraph('p', { kind: 'Stack', id: 'stack-uuid' }, [second, first], false, new Set()))).toBe(
    layoutStructure(graph),
  );
  expect(JSON.stringify(graph)).not.toContain('unrelated');
  const expanded = ownerGraph('p', { kind: 'Stack', id: 'stack-uuid' }, [first, second], false, new Set([group.id]));
  expect(expanded.nodes.filter((n) => n.kind === 'container')).toHaveLength(2);
  expect(expanded.edges).toHaveLength(4);
});

it('keeps service observations visible without claiming that tasks are connected', () => {
  const value = network(
    [
      {
        relationship: NetworkRelationship.ServiceNetwork,
        workloadKind: NetworkWorkloadKind.SwarmService,
        workloadId: 'service',
        workloadName: 'worker',
        dockerServiceId: 'docker-service',
        serviceName: 'worker',
        isStale: false,
        workloadProcessing: false,
      },
    ],
    { scope: 'swarm' },
  );
  const graph = ownerGraph('p', { kind: 'SwarmService', id: 'service' }, [value], false, new Set());
  expect(graph.nodes.filter((n) => n.kind === 'container')).toHaveLength(0);
  expect(graph.nodes.find((n) => n.kind === 'service')?.subtitle).toBe('1 service network');
  expect(graph.edges[0].service).toBe(true);
});

it('preserves both nodes’ observations when merging the same overlay network', () => {
  const values = ['node-a', 'node-b'].map((dockerNodeId) =>
    network([attachment(dockerNodeId, { ...stack, dockerNodeId, dockerContainerId: 'same-docker-id' })], {
      scope: 'swarm',
      dockerNodeId,
      containers: {
        'same-docker-id': {
          name: dockerNodeId,
          endpointId: null,
          macAddress: null,
          ipV4Address: '10.0.0.2/24',
          ipv6Address: null,
        },
      },
    }),
  );
  const graph = ownerGraph('p', { kind: 'Stack', id: 'stack-uuid' }, values, false, new Set());
  const networks = graph.nodes.filter((node) => node.kind === 'network');
  expect(networks).toHaveLength(1);
  expect(networks[0].network?.dockerNodeId).toBeNull();
  expect(networks[0].network?.containers['same-docker-id']).toBeUndefined();
  expect(networks[0].network?.usage?.entries.map((entry) => entry.dockerNodeId)).toEqual(['node-a', 'node-b']);
  expect(graph.nodes.find((node) => node.kind === 'stack')?.entries).toHaveLength(2);
});

it('bounds the whole workload graph and keeps saved use separate from observations', () => {
  const refs = network([
    {
      ...stack,
      relationship: NetworkRelationship.SavedReference,
      reference: 'proxy',
      isStale: false,
      workloadProcessing: false,
    },
  ]);
  const owner = { kind: 'Stack', id: 'stack-uuid' } as const;
  expect(ownerGraph('p', owner, [refs], false, new Set()).nodes).toHaveLength(0);
  expect(ownerGraph('p', owner, [refs], true, new Set()).edges[0].saved).toBe(true);
  const values = Array.from({ length: 60 }, (_, i) => network([attachment(`app-${i}`, stack)], { id: `network-${i}` }));
  const graph = ownerGraph('p', owner, values, false, new Set());
  expect(graph.truncated).toBe(true);
  expect(graph.edges).toHaveLength(MAX_MAP_ENTRIES);
  expect(
    ownerGraph('p', owner, [{ ...refs, usage: { ...refs.usage!, truncated: true } }], false, new Set()).truncated,
  ).toBe(true);
});

it('draws only authorized attachments; saved references are opt-in and distinct', () => {
  const value = network(
    [
      attachment('app'),
      {
        isStale: false,
        workloadProcessing: false,
        relationship: NetworkRelationship.SavedReference,
        ...stack,
        reference: 'proxy',
      },
    ],
    {
      containers: {
        hidden: { name: 'hidden-endpoint', endpointId: null, ipV4Address: null, ipv6Address: null, macAddress: null },
      },
    },
  );
  const observed = networkGraph('p', value, false, new Set());
  expect(observed.nodes.map((node) => node.label)).toEqual(['proxy', 'app']);
  expect(observed.edges).toHaveLength(1);
  const configured = networkGraph('p', value, true, new Set());
  expect(configured.edges.filter((edge) => edge.saved)).toHaveLength(1);
  expect(configured.nodes.find((node) => node.kind === 'reference')?.entries[0].reference).toBe('proxy');
});

it('collapses only observed Stack members and preserves member edges on expansion', () => {
  const value = network([attachment('app', stack), attachment('db', stack)]);
  const collapsed = networkGraph('p', value, false, new Set());
  const group = collapsed.nodes.find((node) => node.kind === 'stack')!;
  expect(group.subtitle).toBe('2 connected containers');
  expect(collapsed.edges.map((edge) => edge.target)).toEqual([group.id]);
  const expanded = networkGraph('p', value, false, new Set([group.id]));
  expect(expanded.nodes.filter((node) => node.parentId === group.id)).toHaveLength(2);
  expect(expanded.edges).toHaveLength(2);
  expect(expanded.edges.some((edge) => edge.target === group.id)).toBe(false);
});

it('uses node and platform identity, deduplicating only the same endpoint evidence', () => {
  const value = network([
    attachment('same', { dockerNodeId: 'a' }),
    attachment('same', { dockerNodeId: 'b' }),
    attachment('same', { dockerNodeId: 'a' }),
  ]);
  const a = networkGraph('a', value, false, new Set());
  const b = networkGraph('b', value, false, new Set());
  expect(a.nodes.filter((node) => node.kind === 'container')).toHaveLength(2);
  expect(a.nodes.some((node) => b.nodes.some((other) => other.id === node.id))).toBe(false);
});

it('summarizes Swarm observations without claiming desired replicas are attached', () => {
  const value = network([
    attachment('task', { workloadKind: NetworkWorkloadKind.SwarmService, workloadId: 'svc', workloadName: 'Web' }),
  ]);
  expect(networkGraph('p', value, false, new Set()).nodes[1]).toMatchObject({
    kind: 'service',
    subtitle: '1 connected container',
    expanded: false,
  });
});

it('bounds graphs and relayouts topology changes only', () => {
  const value = network(Array.from({ length: 200 }, (_, index) => attachment(String(index))));
  const graph = networkGraph('p', value, false, new Set());
  expect(graph.truncated).toBe(true);
  expect(graph.nodes).toHaveLength(MAX_MAP_ENTRIES + 1);
  expect(layoutStructure(networkGraph('p', { ...value, name: 'renamed', inUse: false }, false, new Set()))).toBe(
    layoutStructure(graph),
  );
  expect(layoutStructure(networkGraph('p', network(value.usage!.entries.slice(1, 4)), false, new Set()))).not.toBe(
    layoutStructure(graph),
  );
});

it('lays out expanded groups deterministically with real ELK and no child overlaps', async () => {
  const value = network([attachment('app', stack), attachment('db', stack), attachment('proxy')]);
  const collapsed = networkGraph('p', value, false, new Set());
  const group = collapsed.nodes.find((node) => node.kind === 'stack')!;
  const graph = networkGraph('p', value, false, new Set([group.id]));
  const structure = JSON.parse(layoutStructure(graph));
  const elk = new ELK();
  const first = layoutPositions(await elk.layout(elkGraph(structure)));
  expect(first).toEqual(layoutPositions(await elk.layout(elkGraph(structure))));
  expect(first).toHaveLength(graph.nodes.length);
  const parent = first.find((node) => node.id === group.id)!;
  const children = graph.nodes
    .filter((node) => node.parentId === group.id)
    .map((node) => first.find((position) => position.id === node.id)!);
  for (const child of children) {
    expect(child.y).toBeGreaterThanOrEqual(56);
    expect(child.x + child.width).toBeLessThanOrEqual(parent.width);
    expect(child.y + child.height).toBeLessThanOrEqual(parent.height);
  }
  const [a, b] = children;
  expect(a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y).toBe(true);
});

it('shows an application on two networks without inventing network-to-network routes', async () => {
  const { workloadGraph, highlightedNodes } = await import('./model');
  const container = {
    isStale: false,
    containerState: 'Running',
    workloadProcessing: false,
    containerId: 'uuid-app',
    containerName: 'app',
    dockerContainerId: 'docker-app',
    dockerNodeId: null,
  };
  const networking = {
    container,
    mode: 'networks',
    endpoints: [
      { attached: true, networkId: 'proxy', name: 'proxy', addresses: ['172.20.0.2'], aliases: ['app'] },
      { attached: true, networkId: 'backend', name: 'backend', addresses: ['172.21.0.2'], aliases: ['app'] },
    ],
    publishedPorts: [],
    observedAt: 1,
    truncated: false,
  };
  const graph = workloadGraph('p', container, networking, [
    network([]),
    network([], { id: 'backend', name: 'backend' }),
  ]);
  expect(graph.nodes).toHaveLength(3);
  expect(graph.edges).toHaveLength(2);
  const app = graph.nodes.find((node) => node.kind === 'container')!;
  expect(graph.edges.every((edge) => edge.target === app.id)).toBe(true);
  expect(graph.nodes.find((node) => node.network?.id === 'proxy')?.network?.usage?.entries[0].containerId).toBe(
    'uuid-app',
  );
  expect(
    workloadGraph(
      'p',
      container,
      { ...networking, endpoints: networking.endpoints.map((endpoint) => ({ ...endpoint, attached: false })) },
      [network([])],
    ).edges,
  ).toEqual([]);
  expect(
    workloadGraph(
      'p',
      container,
      { ...networking, endpoints: networking.endpoints.map((endpoint) => ({ ...endpoint, attached: null })) },
      [network([])],
    ).edges,
  ).toEqual([]);
  expect(highlightedNodes(graph, app.id).size).toBe(3);
  expect(highlightedNodes(graph, graph.nodes[1].id).size).toBe(2);
  expect(workloadGraph('p', container, { ...networking, mode: 'host' }, [network([])]).edges).toEqual([]);
  expect(workloadGraph('p', container, { ...networking, mode: 'shared' }, [network([])]).edges).toEqual([]);
  expect(workloadGraph('p', container, networking, [network([], { dockerNodeId: 'another-node' })]).edges).toEqual([]);
});

it('groups observed Swarm service membership without inventing task attachments', () => {
  const service: NetworkUsageEntry = {
    isStale: false,
    workloadProcessing: false,
    relationship: NetworkRelationship.ServiceNetwork,
    dockerServiceId: 'service-id',
    serviceName: 'web',
    ...stack,
  };
  const value = network([service], { scope: 'swarm' });
  const collapsed = networkGraph('p', value, false, new Set());
  const group = collapsed.nodes.find((node) => node.kind === 'stack')!;
  expect(group.subtitle).toBe('1 service network · 0 task attachments');
  expect(collapsed.nodes.filter((node) => node.kind === 'container')).toEqual([]);
  expect(collapsed.edges).toHaveLength(1);
  expect(collapsed.edges[0].service).toBe(true);
  const expanded = networkGraph('p', value, false, new Set([group.id]));
  expect(expanded.nodes.find((node) => node.kind === 'service')?.parentId).toBe(group.id);
  expect(expanded.edges).toHaveLength(1);
  expect(expanded.edges[0].target).not.toBe(group.id);
});

it('preserves container state and freshness when focusing its connections', () => {
  const container = {
    containerId: 'c',
    dockerContainerId: 'docker-c',
    containerName: 'app',
    containerState: 'Paused',
    isStale: false,
    workloadProcessing: false,
  };
  const networking = {
    container,
    mode: 'networks',
    endpoints: [],
    publishedPorts: [],
    observedAt: 1,
    truncated: false,
  };
  const node = workloadGraph('p', container, networking, []).nodes[0];
  expect(node.entries[0].containerState).toBe('Paused');
  expect(node.isStale).toBe(false);
  expect(workloadGraph('p', { ...container, isStale: true }, networking, []).nodes[0].isStale).toBe(true);
});
