import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { NetworkWorkloadKind } from '@/api/generated/api.types';
import { WorkloadLink } from './workload-link';

it.each([
  [NetworkWorkloadKind.Deployment, 'Healthy', 'bg-green-500', 'deployments'],
  [NetworkWorkloadKind.Stack, 'Stopped', 'bg-gray-500', 'stacks'],
  [NetworkWorkloadKind.SwarmService, 'Degraded', 'bg-orange-500', 'swarm-services'],
  [NetworkWorkloadKind.SwarmService, 'Unknown', 'bg-violet-400', 'swarm-services'],
])('shows %s status %s with its configuration link', (kind, status, color, path) => {
  const { container } = render(
    <MemoryRouter>
      <WorkloadLink
        entry={{
          workloadId: 'owner',
          workloadName: 'Application',
          workloadKind: kind,
          workloadStatus: status,
          workloadProcessing: false,
        }}
      />
    </MemoryRouter>,
  );
  expect(screen.getByRole('link', { name: 'Application' })).toHaveAttribute('href', `/${path}/edit/owner/#config`);
  expect(container.querySelector('.rounded-full')).toHaveClass(color);
});

it('shows an operation indicator while the workload is processing', () => {
  const { container } = render(
    <MemoryRouter>
      <WorkloadLink
        entry={{
          workloadId: 'owner',
          workloadName: 'Application',
          workloadKind: NetworkWorkloadKind.Stack,
          workloadStatus: 'Healthy',
          workloadProcessing: true,
        }}
      />
    </MemoryRouter>,
  );
  expect(container.querySelector('.animate-spin')).toBeInTheDocument();
});

it('omits unresolved owners', () => {
  const { container } = render(
    <MemoryRouter>
      <WorkloadLink entry={{ workloadProcessing: false }} />
    </MemoryRouter>,
  );
  expect(container).toBeEmptyDOMElement();
});
