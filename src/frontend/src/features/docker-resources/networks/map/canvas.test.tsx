import { act, render, screen } from '@testing-library/react';
import type { ElkNode } from 'elkjs';
import NetworkCanvas from './canvas';
import type { NetworkGraph } from './model';

const state = vi.hoisted(() => ({
  jobs: [] as { resolve: (value: ElkNode) => void }[],
  workers: [] as { terminate: ReturnType<typeof vi.fn> }[],
  props: {} as Record<string, unknown>,
}));
vi.mock('elkjs/lib/elk-api', () => ({
  default: class {
    layout() {
      return new Promise<ElkNode>((resolve) => state.jobs.push({ resolve }));
    }
  },
}));
vi.mock('@xyflow/react', () => ({
  Position: { Top: 'top', Bottom: 'bottom' },
  Background: () => null,
  Controls: () => null,
  Handle: () => null,
  ReactFlow: (props: { nodes: { id: string }[] }) => {
    state.props = props;
    return (
      <div data-testid="canvas">
        {props.nodes.map((node) => (
          <span key={node.id}>{node.id}</span>
        ))}
      </div>
    );
  },
}));
const graph: NetworkGraph = {
  nodes: [{ id: 'network', kind: 'network', label: 'Network', subtitle: 'bridge', entries: [] }],
  edges: [],
  truncated: false,
};

beforeEach(() => {
  state.jobs = [];
  state.workers = [];
  vi.stubGlobal(
    'Worker',
    class {
      terminate = vi.fn();
      constructor() {
        state.workers.push(this);
      }
    },
  );
});
afterAll(() => vi.unstubAllGlobals());

it('preserves layout for presentation updates and ignores cancelled layout results', async () => {
  const props = { graph, onToggle: vi.fn(), onSelect: vi.fn(), reset: 0 };
  const view = render(<NetworkCanvas {...props} />);
  expect(state.jobs).toHaveLength(1);
  view.rerender(
    <NetworkCanvas {...props} graph={{ ...graph, nodes: [{ ...graph.nodes[0], label: 'Renamed', inUse: true }] }} />,
  );
  expect(state.jobs).toHaveLength(1);
  const changed = { ...graph, nodes: [{ ...graph.nodes[0], id: 'replacement' }] };
  view.rerender(<NetworkCanvas {...props} graph={changed} />);
  expect(state.workers[0].terminate).toHaveBeenCalledOnce();
  expect(state.jobs).toHaveLength(2);
  await act(async () => state.jobs[1].resolve({ id: 'root', children: [{ id: 'replacement', x: 12, y: 12 }] }));
  expect(screen.getByTestId('canvas')).toHaveTextContent('replacement');
  await act(async () => state.jobs[0].resolve({ id: 'root', children: [{ id: 'network', x: 12, y: 12 }] }));
  expect(screen.getByTestId('canvas')).toHaveTextContent('replacement');
  expect(state.props).toMatchObject({
    zIndexMode: 'manual',
    nodesDraggable: false,
    nodesConnectable: false,
    edgesReconnectable: false,
    deleteKeyCode: null,
  });
  view.unmount();
  expect(state.workers[1].terminate).toHaveBeenCalledOnce();
});

it('draws configured container connections with dashed labels and restores solid live edges without relayout', async () => {
  const configured: NetworkGraph = {
    ...graph,
    nodes: [...graph.nodes, { id: 'container', kind: 'container', label: 'App', subtitle: 'Container', entries: [] }],
    edges: [
      { id: 'connection', source: 'network', target: 'container', saved: true, label: 'Configured · Not attached' },
    ],
  };
  const props = { graph: configured, onToggle: vi.fn(), onSelect: vi.fn(), reset: 0 };
  const view = render(<NetworkCanvas {...props} />);
  await act(async () =>
    state.jobs[0].resolve({
      id: 'root',
      children: [
        { id: 'network', x: 12, y: 12 },
        { id: 'container', x: 12, y: 200 },
      ],
    }),
  );
  expect(state.props).toMatchObject({
    edges: [{ label: 'Configured · Not attached', style: { strokeDasharray: '5 5' } }],
  });
  view.rerender(
    <NetworkCanvas
      {...props}
      graph={{ ...configured, edges: [{ ...configured.edges[0], saved: false, label: undefined }] }}
    />,
  );
  expect(state.props).toMatchObject({ edges: [{ label: undefined }] });
  expect(state.props).not.toMatchObject({ edges: [{ style: { strokeDasharray: expect.anything() } }] });
  expect(state.jobs).toHaveLength(1);
});
