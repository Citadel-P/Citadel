import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation, useNavigate } from 'react-router';
import { ResourceCopyKind, ResourceCopyEligibilityStatus } from '@/api/generated/api.types';
import { renderCitadel as renderWithProviders } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { DuplicateWorkload } from './duplicate-workload';

function renderCitadel(...[ui, options]: Parameters<typeof renderWithProviders>) {
  const fake = new FakeRealtimeConnection();
  return renderWithProviders(ui, {
    groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
    ...options,
  });
}

const available = { status: ResourceCopyEligibilityStatus.Available, reason: null };
const id = '019d0000-0001-7000-8001-000000000001';
const snapshot = {
  backupRunId: 'run',
  backupRunItemId: 'item',
  policyId: 'policy',
  repositoryId: 'repo',
  policyName: 'Daily',
  volume: 'data',
  dockerNodeId: null,
  completedAt: '2026-10-15T10:00:00Z',
};
const preview = {
  id,
  sourceName: 'WordPress',
  platformId: 'platform',
  platformName: 'Docker',
  name: 'WordPress-copy',
  volumes: [{ source: 'data', target: 'new-data', dockerNodeId: null, snapshots: [snapshot] }],
  blockers: [],
  warnings: ['The copy stays undeployed.'],
};
const progress = {
  id,
  kind: ResourceCopyKind.Stack,
  sourceId: 'source',
  name: 'WordPress-copy',
  platformId: 'platform',
  status: 'Restoring',
  destinationId: null,
  message: 'Restoring data',
  restores: [{ id: 'restore', source: 'data', target: 'new-data', status: 'Running', message: null }],
};
beforeEach(() => {
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', () => HttpResponse.json(preview)),
    http.get('http://localhost/api/v1/resource-copies/targets', () =>
      HttpResponse.json({
        sourcePlatformId: 'platform',
        sourcePlatformName: 'Docker',
        sourceNodeId: null,
        swarm: false,
        platforms: [
          { id: 'platform', name: 'Docker' },
          { id: 'destination', name: 'Destination' },
        ],
      }),
    ),
  );
});

function Location() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <button onClick={() => navigate(-1)}>Go back</button>
      <output aria-label="Page URL">
        {location.pathname}
        {location.search}
      </output>
    </>
  );
}
function render(route = '/stacks/edit/source/') {
  return renderCitadel(
    <>
      <DuplicateWorkload
        kind={ResourceCopyKind.Stack}
        sourceId="source"
        sourceName="WordPress"
        copyEligibility={available}
        configurationHref="/stacks/add?duplicateFrom=source"
      />
      <Location />
    </>,
    { route },
  );
}

it('uses three reviewed steps and starts only after explicit confirmation', async () => {
  const emptyRead = vi.fn(() => new HttpResponse(null, { status: 404 }));
  server.use(http.get('http://localhost/api/v1/resource-copies/', emptyRead));
  const requests: unknown[] = [];
  const start = vi.fn(() => HttpResponse.json(progress));
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', async ({ request }) => {
      requests.push(await request.json());
      return HttpResponse.json(preview);
    }),
    http.post(`http://localhost/api/v1/resource-copies/${id}/start`, start),
    http.get(`http://localhost/api/v1/resource-copies/${id}`, () => HttpResponse.json(progress)),
  );
  const { user } = render();
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await waitFor(() =>
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
      'aria-disabled',
      'true',
    ),
  );
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('WordPress-copy');
  await user.click(screen.getByRole('combobox', { name: 'Data source' }));
  await user.click(screen.getByRole('option', { name: 'Use an existing backup' }));
  await user.click(screen.getByRole('button', { name: 'Choose backups' }));
  await user.click(await screen.findByRole('combobox', { name: 'data' }));
  await user.click(screen.getByRole('option', { name: /Daily/ }));
  await user.click(screen.getByRole('button', { name: 'Review copy' }));
  expect(await screen.findByText('New volume: new-data')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Create copy' })).toBeDisabled();
  expect(start).not.toHaveBeenCalled();
  await user.click(screen.getByRole('checkbox'));
  await user.click(screen.getByRole('button', { name: 'Create copy' }));
  expect(await screen.findByText('Restoring data')).toBeVisible();
  expect(requests).toEqual([
    {
      dataMode: 'ExistingBackup',
      consistencyAcknowledged: false,
      kind: 'Stack',
      sourceId: 'source',
      name: 'WordPress-copy',
      selections: [],
    },
    {
      dataMode: 'ExistingBackup',
      consistencyAcknowledged: false,
      kind: 'Stack',
      sourceId: 'source',
      name: 'WordPress-copy',
      selections: [{ volume: 'data', backupRunId: 'run', backupRunItemId: 'item' }],
    },
  ]);
  expect(start).toHaveBeenCalledTimes(1);
  expect(emptyRead).not.toHaveBeenCalled();
  expect(screen.getByLabelText('Page URL')).toHaveTextContent(`copyOperation=${id}`);
  await user.click(screen.getAllByRole('button', { name: 'Close' })[0]);
  expect(screen.getByLabelText('Page URL')).toHaveTextContent('/stacks/edit/source/');
});

it('resumes a completed operation after refresh without restarting or redirecting', async () => {
  const start = vi.fn();
  const check = vi.fn(() => new HttpResponse(null, { status: 403 }));
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', check),
    http.post(`http://localhost/api/v1/resource-copies/${id}/start`, start),
    http.get(`http://localhost/api/v1/resource-copies/${id}`, () =>
      HttpResponse.json({ ...progress, status: 'Succeeded', destinationId: 'copy', message: 'Copy created.' }),
    ),
  );
  const { user } = render(`/stacks/edit/source/?copyOperation=${id}`);
  expect(await screen.findByRole('link', { name: 'Open stack' })).toHaveAttribute('href', '/stacks/edit/copy/');
  expect(start).not.toHaveBeenCalled();
  expect(screen.getByLabelText('Page URL')).toHaveTextContent('/stacks/edit/source/');
  await user.click(screen.getAllByRole('button', { name: 'Close' })[0]);
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  expect(await screen.findByRole('link', { name: 'Open stack' })).toBeVisible();
  expect(check).not.toHaveBeenCalled();
});

it('revalidates storage inside the wizard when it changes after opening the menu', async () => {
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', () =>
      HttpResponse.json({ ...preview, volumes: [], blockers: ['Bind mounts cannot be copied automatically.'] }),
    ),
  );
  const { user } = render();
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await waitFor(() =>
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
      'aria-disabled',
      'true',
    ),
  );
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  await user.click(screen.getByRole('button', { name: 'Review volumes' }));
  expect(await screen.findByText('Bind mounts cannot be copied automatically.')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Review copy' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Configuration only' })).toBeEnabled();
});

it.each([
  [ResourceCopyKind.Deployment, 'deployments'],
  [ResourceCopyKind.Stack, 'stacks'],
  [ResourceCopyKind.SwarmService, 'swarm-services'],
])('offers configuration-only duplication when %s has no copyable volumes', async (kind, path) => {
  const start = vi.fn();
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', () =>
      HttpResponse.json({
        ...preview,
        volumes: [],
        blockers: ['No declared named volumes were found. Use Configuration only for this resource.'],
      }),
    ),
    http.post(`http://localhost/api/v1/resource-copies/${id}/start`, start),
  );
  const { user } = renderCitadel(
    <>
      <DuplicateWorkload
        kind={kind}
        sourceId="source"
        sourceName="redis"
        copyEligibility={{
          status: ResourceCopyEligibilityStatus.Unavailable,
          reason: 'No declared named volumes were found. Use Configuration only for this resource.',
        }}
        configurationHref={`/${path}/add?duplicateFrom=source`}
      />
      <Location />
    </>,
    { route: `/${path}/edit/source/` },
  );
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  expect(
    await screen.findByText('No declared named volumes were found. Use Configuration only for this resource.'),
  ).toBeVisible();
  expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await user.click(screen.getByRole('menuitem', { name: /^Configuration only/ }));
  await waitFor(() => expect(screen.getByLabelText('Page URL')).toHaveTextContent(`/${path}/add?duplicateFrom=source`));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(start).not.toHaveBeenCalled();
});

it('keeps configuration-only duplication on the existing resource form', async () => {
  const { user } = render();
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /^Configuration only/ }));
  await waitFor(() => expect(screen.getByLabelText('Page URL')).toHaveTextContent('/stacks/add?duplicateFrom=source'));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it.each([ResourceCopyKind.Deployment, ResourceCopyKind.Stack, ResourceCopyKind.SwarmService])(
  'uses model updates for %s eligibility without requesting a preview',
  async (kind) => {
    const check = vi.fn(() => HttpResponse.json(preview));
    server.use(http.post('http://localhost/api/v1/resource-copies/preview', check));
    const props = { kind, sourceId: 'source', sourceName: 'Example', configurationHref: '/stacks/add' };
    const { user, rerender } = renderCitadel(<DuplicateWorkload {...props} />);
    await user.click(screen.getByRole('button', { name: 'Duplicate' }));
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByText(/Copy availability could not be checked/)).toBeVisible();
    rerender(
      <DuplicateWorkload
        {...props}
        copyEligibility={{
          status: ResourceCopyEligibilityStatus.Unavailable,
          reason: 'Each volume needs an available backup you have permission to restore.',
        }}
      />,
    );
    expect(screen.getByText(/Each volume needs an available backup/)).toBeVisible();
    rerender(<DuplicateWorkload {...props} copyEligibility={available} />);
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
      'aria-disabled',
      'true',
    );
    await user.keyboard('{Escape}');
    await user.click(screen.getByRole('button', { name: 'Duplicate' }));
    expect(check).not.toHaveBeenCalled();
    // Permission revocation can arrive while the dropdown remains open.
    rerender(
      <DuplicateWorkload
        {...props}
        copyEligibility={{
          status: ResourceCopyEligibilityStatus.Unavailable,
          reason: 'You no longer have permission to restore these backups.',
        }}
      />,
    );
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('menuitem', { name: /^Configuration only/ })).not.toHaveAttribute('aria-disabled', 'true');
    expect(check).not.toHaveBeenCalled();
  },
);

it('uses a valid suggested name for the wizard when the resource name contains spaces', async () => {
  const check = vi.fn(async ({ request }: { request: Request }) => {
    expect(await request.json()).toMatchObject({ name: 'Redis-cache-copy', selections: [] });
    return HttpResponse.json(preview);
  });
  server.use(http.post('http://localhost/api/v1/resource-copies/preview', check));
  const { user } = renderCitadel(
    <DuplicateWorkload
      kind={ResourceCopyKind.Deployment}
      sourceId="source"
      sourceName="Redis cache"
      copyEligibility={available}
      configurationHref="/deployments/add?duplicateFrom=source"
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await waitFor(() =>
    expect(screen.getByRole('menuitem', { name: /Configuration \+ data/ })).not.toHaveAttribute(
      'aria-disabled',
      'true',
    ),
  );
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('Redis-cache-copy');
  expect(check).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Review volumes' }));
  await waitFor(() => expect(check).toHaveBeenCalledTimes(1));
});

it('copies current data without backups and requires a consistency confirmation before review', async () => {
  const requests: unknown[] = [];
  const start = vi.fn(() => HttpResponse.json(progress));
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', async ({ request }) => {
      const body = (await request.json()) as { consistencyAcknowledged: boolean };
      requests.push(body);
      return HttpResponse.json({
        ...preview,
        volumes: [{ ...preview.volumes[0], snapshots: [] }],
        blockers: body.consistencyAcknowledged ? [] : ['Confirm source consistency before copying current data.'],
      });
    }),
    http.post(`http://localhost/api/v1/resource-copies/${id}/start`, start),
    http.get(`http://localhost/api/v1/resource-copies/${id}`, () => HttpResponse.json(progress)),
  );
  const { user } = render();
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  expect(screen.getByRole('combobox', { name: 'Data source' })).toHaveTextContent('Copy current data');
  await user.click(screen.getByRole('button', { name: 'Review volumes' }));
  expect(await screen.findByText('Copy the current volume data')).toBeVisible();
  expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Review copy' })).toBeDisabled();
  await user.click(screen.getByRole('checkbox', { name: /The source data is ready/ }));
  await user.click(screen.getByRole('button', { name: 'Review copy' }));
  expect(await screen.findByText('New volume: new-data')).toBeVisible();
  expect(start).not.toHaveBeenCalled();
  expect(requests).toEqual(
    [false, true].map((consistencyAcknowledged) => ({
      kind: 'Stack',
      sourceId: 'source',
      name: 'WordPress-copy',
      dataMode: 'CurrentData',
      consistencyAcknowledged,
      selections: [],
    })),
  );
  await user.click(screen.getByRole('checkbox', { name: /I’ve reviewed/ }));
  await user.click(screen.getByRole('button', { name: 'Create copy' }));
  await waitFor(() => expect(start).toHaveBeenCalledTimes(1));
});

it('updates progress over realtime without polling and clears the completed link before opening the copy', async () => {
  const fake = new FakeRealtimeConnection();
  const read = vi.fn(() => HttpResponse.json(progress));
  const emptyRead = vi.fn(() => new HttpResponse(null, { status: 404 }));
  server.use(
    http.get(`http://localhost/api/v1/resource-copies/${id}`, read),
    http.get('http://localhost/api/v1/resource-copies/', emptyRead),
  );
  const { user } = renderCitadel(
    <>
      <DuplicateWorkload
        kind={ResourceCopyKind.Stack}
        sourceId="source"
        sourceName="WordPress"
        copyEligibility={available}
        configurationHref="/stacks/add?duplicateFrom=source"
      />
      <Location />
    </>,
    {
      route: `/stacks/edit/source/?copyOperation=${id}`,
      groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
    },
  );
  await screen.findByText('Restoring data');
  expect(screen.getByText('Restoring').querySelector('.animate-spin')).toBeInTheDocument();
  expect(screen.getByText('Running').querySelector('.animate-spin')).toBeInTheDocument();
  await waitFor(() => expect(fake.listenerCount('ResourceCopyInfoUpdated')).toBe(1));
  act(() => fake.emit('ResourceCopyInfoUpdated', { ...progress, id: 'unrelated', message: 'Wrong copy' }));
  expect(screen.queryByText('Wrong copy')).not.toBeInTheDocument();
  await new Promise((resolve) => setTimeout(resolve, 2200));
  expect(read).toHaveBeenCalledTimes(1);
  // Reconnect rejoins the same group; its snapshot replaces progress without GET polling.
  act(() => fake.reconnecting());
  act(() => fake.reconnected());
  await waitFor(() => expect(fake.listenerCount('ResourceCopyInfoUpdated')).toBe(1));
  act(() =>
    fake.emit('ResourceCopyInfoUpdated', {
      ...progress,
      status: 'Succeeded',
      destinationId: 'copy',
      message: 'Copy created.',
      restores: [{ ...progress.restores[0], status: 'Succeeded' }],
    }),
  );
  const link = await screen.findByRole('link', { name: 'Open stack' });
  expect(screen.getAllByText('Succeeded').every((badge) => !badge.querySelector('.animate-spin'))).toBe(true);
  expect(read).toHaveBeenCalledTimes(1);
  await user.click(link);
  await user.click(screen.getByRole('button', { name: 'Go back' }));
  expect(screen.getByLabelText('Page URL')).not.toHaveTextContent('copyOperation');
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('WordPress-copy');
  expect(screen.queryByText('Copy could not continue')).not.toBeInTheDocument();
  expect(emptyRead).not.toHaveBeenCalled();
});

it('allows a fresh copy when a saved progress link is no longer available', async () => {
  server.use(http.get(`http://localhost/api/v1/resource-copies/${id}`, () => new HttpResponse(null, { status: 404 })));
  const { user } = render(`/stacks/edit/source/?copyOperation=${id}`);
  await screen.findByText('Copy could not continue');
  await user.click(screen.getByRole('button', { name: 'New copy' }));
  expect(screen.getByRole('textbox', { name: 'Name' })).toBeVisible();
  expect(screen.queryByText('Copy could not continue')).not.toBeInTheDocument();
});

it('selects an authorized destination and uses existing backups for cross-platform copies', async () => {
  const requests: unknown[] = [];
  server.use(
    http.post('http://localhost/api/v1/resource-copies/preview', async ({ request }) => {
      requests.push(await request.json());
      return HttpResponse.json({ ...preview, platformId: 'destination', platformName: 'Destination' });
    }),
  );
  const { user } = render();
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  const platform = await screen.findByRole('combobox', { name: 'Destination platform' });
  await waitFor(() => expect(platform).toHaveTextContent('Docker'));
  await user.click(platform);
  await user.click(screen.getByRole('option', { name: 'Destination' }));
  expect(screen.getByRole('combobox', { name: 'Data source' })).toHaveTextContent('Use an existing backup');
  await user.click(screen.getByRole('combobox', { name: 'Data source' }));
  expect(screen.getByRole('option', { name: 'Copy current data' })).toHaveAttribute('aria-disabled', 'true');
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Choose backups' }));
  await screen.findByRole('combobox', { name: 'data' });
  expect(requests[0]).toMatchObject({ destinationPlatformId: 'destination', dataMode: 'ExistingBackup' });
  await user.click(screen.getByRole('combobox', { name: 'data' }));
  await user.click(screen.getByRole('option', { name: /Daily/ }));
  await user.click(screen.getByRole('button', { name: 'Review copy' }));
  expect(await screen.findByText('New volume: new-data')).toBeVisible();
  expect(screen.getByText('Destination')).toBeVisible();
});

it('requires an active destination Swarm node and resets it when the platform changes', async () => {
  server.use(
    http.get('http://localhost/api/v1/resource-copies/targets', () =>
      HttpResponse.json({
        sourcePlatformId: 'platform',
        sourcePlatformName: 'Source Swarm',
        sourceNodeId: 'sourceNode',
        swarm: true,
        platforms: [
          { id: 'platform', name: 'Source Swarm' },
          { id: 'destination', name: 'Destination Swarm' },
        ],
      }),
    ),
    http.get('http://localhost/api/v1/platforms/:platformId/swarm/nodes', () =>
      HttpResponse.json({
        items: [
          { id: 'targetNode', hostname: 'Target worker', isStale: false, status: 'Ready', availability: 'Active' },
          { id: 'drainedNode', hostname: 'Drained worker', isStale: false, status: 'Ready', availability: 'Drain' },
        ],
      }),
    ),
  );
  const { user } = renderCitadel(
    <DuplicateWorkload
      kind={ResourceCopyKind.SwarmService}
      sourceId="source"
      sourceName="Redis"
      copyEligibility={available}
      configurationHref="/swarm-services/add?duplicateFrom=source"
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Duplicate' }));
  await user.click(screen.getByRole('menuitem', { name: /Configuration \+ data/ }));
  const platform = await screen.findByRole('combobox', { name: 'Destination platform' });
  await waitFor(() => expect(platform).toHaveTextContent('Source Swarm'));
  await user.click(platform);
  await user.click(screen.getByRole('option', { name: 'Destination Swarm' }));
  expect(screen.getByRole('button', { name: 'Choose backups' })).toBeDisabled();
  const node = screen.getByRole('combobox', { name: 'Destination node' });
  await waitFor(() => expect(node).toBeEnabled());
  await user.click(node);
  expect(screen.queryByRole('option', { name: 'Drained worker' })).not.toBeInTheDocument();
  await user.click(await screen.findByRole('option', { name: 'Target worker' }));
  expect(screen.getByRole('button', { name: 'Choose backups' })).toBeEnabled();
  await user.click(platform);
  await user.click(screen.getByRole('option', { name: 'Source Swarm' }));
  await user.click(platform);
  await user.click(screen.getByRole('option', { name: 'Destination Swarm' }));
  expect(screen.getByRole('button', { name: 'Choose backups' })).toBeDisabled();
});
