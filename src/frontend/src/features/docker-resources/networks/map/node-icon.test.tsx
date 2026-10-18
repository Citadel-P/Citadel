import { render, screen } from '@testing-library/react';
import { NetworkRelationship, NetworkWorkloadKind } from '@/api/generated/api.types';
import { MapNodeIcon } from './node-icon';
import type { MapNode } from './model';

const node: MapNode = {
  id: 'app',
  kind: 'container',
  label: 'app',
  subtitle: '',
  entries: [
    {
      relationship: NetworkRelationship.Attachment,
      isStale: false,
      workloadProcessing: false,
      workloadKind: NetworkWorkloadKind.Deployment,
      workloadStatus: 'Healthy',
      containerState: 'Paused',
    },
  ],
};

it('uses the container state for an individual container, even when its deployment is healthy', () => {
  render(<MapNodeIcon node={node} className="text-muted-foreground" />);
  expect(screen.getByRole('img', { name: 'Paused' }).querySelector('svg')).toHaveClass('text-orange-500');
});

it('uses the workload status for a group instead of one member’s state', () => {
  render(<MapNodeIcon node={{ ...node, kind: 'stack' }} />);
  expect(screen.getByRole('img', { name: 'Healthy' }).querySelector('svg')).toHaveClass('text-green-500');
});

it('does not show stale container state as current', () => {
  render(<MapNodeIcon node={{ ...node, isStale: true }} />);
  expect(screen.getByRole('img', { name: 'State is out of date' }).querySelector('svg')).toHaveClass('text-gray-400');
});

it('shows unknown when no state is available', () => {
  render(<MapNodeIcon node={{ ...node, entries: [{ ...node.entries[0], containerState: null }] }} />);
  expect(screen.getByRole('img', { name: 'Unknown' }).querySelector('svg')).toHaveClass('text-gray-400');
});

it.each([
  [true, false, 'In use', 'text-green-500'],
  [false, false, 'Unused', 'text-gray-500'],
  [undefined, false, 'Unknown', 'text-gray-400'],
  [true, true, 'State is out of date', 'text-gray-400'],
] as const)('shows network usage honestly (inUse=%s, stale=%s)', (inUse, isStale, label, color) => {
  render(<MapNodeIcon node={{ ...node, kind: 'network', inUse, isStale }} />);
  expect(screen.getByRole('img', { name: label }).querySelector('svg')).toHaveClass(color);
});
