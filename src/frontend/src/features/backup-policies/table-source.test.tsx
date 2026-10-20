import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  BackupPolicyView,
  BackupSourceSpec,
  ResourceControlState,
  VolumeBackupConsistency,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BackupPoliciesTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({
  useProfileDateTimeFormatter: () => (value: unknown) => String(value),
}));

function policy(id: string, source: BackupSourceSpec): BackupPolicyView {
  return {
    id,
    name: `Policy ${id}`,
    source,
    enabled: true,
    controlState: ResourceControlState.Idle,
    cron: null,
    latestRun: null,
    tags: [],
  } as unknown as BackupPolicyView;
}

it('uses resource names and Activities links for workloads, with one lookup per source type', async () => {
  const requests: string[] = [];
  server.use(
    http.get('http://localhost/api/v1/lookup', ({ request }) => {
      const type = new URL(request.url).searchParams.get('TargetResourceType')!;
      requests.push(type);
      const resources = {
        Deployment: [{ id: 'deployment', name: 'forgejo' }],
        Stack: [{ id: 'stack', name: 'wordpress' }],
        SwarmService: [{ id: 'service', name: 'api' }],
      };
      return HttpResponse.json(resources[type as keyof typeof resources]);
    }),
  );
  const { user } = renderCitadel(
    <BackupPoliciesTable
      items={[
        policy('1', { $type: 'Deployment', deploymentId: 'deployment' }),
        policy('2', { $type: 'Stack', stackId: 'stack' }),
        policy('3', { $type: 'SwarmService', swarmServiceId: 'service' }),
        policy('4', { $type: 'Deployment', deploymentId: 'deployment' }),
      ]}
      actions={{}}
      isLoading={false}
    />,
    { route: '/backup-policies' },
  );
  expect(await screen.findByRole('link', { name: 'wordpress' })).toHaveAttribute('href', '/stacks/edit/stack');
  expect(await screen.findByRole('link', { name: 'api' })).toHaveAttribute('href', '/swarm-services/edit/service');
  for (const link of await screen.findAllByRole('link', { name: 'forgejo' })) {
    expect(link).toHaveAttribute('href', '/deployments/edit/deployment');
    expect(link).toHaveClass('table-link');
  }
  expect(requests.sort()).toEqual(['Deployment', 'Stack', 'SwarmService']);
  await user.click(within(screen.getByRole('columnheader', { name: 'Source' })).getByRole('button', { hidden: true }));
  const rows = screen.getAllByRole('row').slice(1);
  expect(within(rows[0]).getByRole('link', { name: 'api' })).toBeVisible();
  expect(within(rows[3]).getByRole('link', { name: 'wordpress' })).toBeVisible();
});

it('keeps deleted or inaccessible workloads readable without inventing a resource name or link', async () => {
  let requests = 0;
  server.use(
    http.get('http://localhost/api/v1/lookup', () => {
      requests++;
      return HttpResponse.json([]);
    }),
  );
  renderCitadel(
    <BackupPoliciesTable
      items={[policy('missing', { $type: 'Deployment', deploymentId: 'missing-id' })]}
      actions={{}}
      isLoading={false}
    />,
  );
  await waitFor(() => expect(requests).toBe(1));
  expect(await screen.findByText('Deployment unavailable')).toBeVisible();
  expect(screen.getAllByRole('link')).toHaveLength(1); // The policy itself remains accessible.
  expect(screen.queryByText(/missing-id/)).not.toBeInTheDocument();
});

it('links volumes to the exact node and retains filesystem and Citadel source labels', () => {
  const volume = {
    $type: 'DockerVolume',
    platformId: 'platform',
    volumeName: 'data space',
    consistency: VolumeBackupConsistency.Live,
  } as const;
  renderCitadel(
    <BackupPoliciesTable
      items={[
        policy('local', volume),
        policy('node', { ...volume, volumeName: 'worker-data', dockerNodeId: 'worker/1' }),
        policy('bind', { $type: 'BindMount', platformId: 'platform', hostPath: '/srv/data' }),
        policy('core', { $type: 'CitadelSystem' }),
      ]}
      actions={{}}
      isLoading={false}
    />,
  );
  expect(screen.getByRole('link', { name: 'data space' })).toHaveAttribute(
    'href',
    '/platforms/platform/volumes/data%20space',
  );
  expect(screen.getByRole('link', { name: 'worker-data' })).toHaveAttribute(
    'href',
    '/platforms/platform/volumes/worker-data?dockerNodeId=worker%2F1',
  );
  expect(screen.getByText('Bind: /srv/data')).toBeVisible();
  expect(screen.getByText('Citadel backup')).toBeVisible();
});
