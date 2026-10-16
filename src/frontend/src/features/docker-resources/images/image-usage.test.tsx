import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ImageUsage } from './image-usage';
import type { DockerDaemonListeners } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { ContainerStateStatus } from '@/api/generated/api.types';
const realtime = vi.hoisted(() => ({ listeners: {} as DockerDaemonListeners }));
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: { id: 'platform-1' } }) }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({
  useDockerDaemonGroup: (_platform: string, listeners: DockerDaemonListeners) => {
    realtime.listeners = listeners;
  },
}));

it('requests the selected node, labels saved references, and links to the workload editor', async () => {
  const requests = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/images/platform-1/image-1/usage', ({ request }) => {
      requests(new URL(request.url).searchParams.get('dockerNodeId'));
      return HttpResponse.json({
        entries: [
          {
            relationship: 'SavedTag',
            workloadKind: 'Deployment',
            workloadId: 'deployment-1',
            workloadName: 'web',
            reference: 'nginx:latest',
            isStale: false,
          },
        ],
        coverageComplete: false,
        limitations: ['Git references may be incomplete.'],
        nextOffset: null,
      });
    }),
  );
  renderCitadel(<ImageUsage imageId="image-1" dockerNodeId="worker-2" />);
  expect(await screen.findByRole('link', { name: 'web' })).toHaveAttribute('href', '/deployments/edit/deployment-1');
  expect(screen.getByText('Saved tag reference: nginx:latest')).toBeVisible();
  expect(screen.queryByText('Git references may be incomplete.')).not.toBeInTheDocument();
  expect(
    screen.getByText('Some image references may be missing. Confirm this image is no longer needed before deleting it.'),
  ).toBeVisible();
  expect(requests).toHaveBeenCalledWith('worker-2');
});

it('does not label an unavailable usage response as no references', async () => {
  server.use(
    http.get('http://localhost/api/v1/images/platform-1/image-1/usage', () => HttpResponse.json({}, { status: 403 })),
  );
  renderCitadel(<ImageUsage imageId="image-1" />);
  expect(await screen.findByText('Image references could not be loaded.')).toBeVisible();
  expect(screen.queryByText(/No known references/)).not.toBeInTheDocument();
});

it('refreshes matching container state patches without reacting to other containers or nodes', async () => {
  let state = 'Running';
  const requests = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/images/platform-1/image-1/usage', () => {
      requests();
      return HttpResponse.json({
        entries: [
          {
            relationship: 'Container',
            containerId: 'container-1',
            containerName: 'web',
            containerState: state,
            isStale: false,
          },
        ],
        coverageComplete: false,
        limitations: [],
        nextOffset: null,
      });
    }),
  );
  renderCitadel(<ImageUsage imageId="image-1" dockerNodeId="worker-2" />);
  expect(await screen.findByText('Container: web (Running)')).toBeVisible();
  await act(async () =>
    realtime.listeners.onContainerStateChange?.([
      { id: 'other', containerId: 'container-2', dockerNodeId: 'worker-2', state: ContainerStateStatus.Exited },
      { id: 'sibling', containerId: 'container-1', dockerNodeId: 'worker-1', state: ContainerStateStatus.Exited },
    ]),
  );
  expect(requests).toHaveBeenCalledTimes(1);
  state = 'Exited';
  await act(async () =>
    realtime.listeners.onContainerStateChange?.([
      { id: 'target', containerId: 'container-1', dockerNodeId: 'worker-2', state: ContainerStateStatus.Exited },
    ]),
  );
  expect(await screen.findByText('Container: web (Exited)')).toBeVisible();
  expect(requests).toHaveBeenCalledTimes(2);
});

it('continues a preview scan and returns to the actual previous cursor', async () => {
  const requests = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/images/platform-1/image-1/usage', ({ request }) => {
      const offset = Number(new URL(request.url).searchParams.get('offset'));
      requests(offset);
      return HttpResponse.json({
        entries: [
          {
            relationship: 'SavedTag',
            workloadId: `workload-${offset}`,
            workloadKind: 'Stack',
            workloadName: `match-${offset}`,
            reference: 'nginx',
            isStale: false,
          },
        ],
        coverageComplete: false,
        limitations: [],
        nextOffset: offset === 137 ? 250 : null,
      });
    }),
  );
  const { user } = renderCitadel(
    <ImageUsage
      imageId="image-1"
      initialUsage={{ entries: [], coverageComplete: false, limitations: [], nextOffset: 137 }}
    />,
  );
  expect(screen.getByText('No matches yet. Continue to check more references.')).toBeVisible();
  expect(requests).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'More references' }));
  expect(await screen.findByRole('link', { name: 'match-137' })).toBeVisible();
  await waitFor(() => expect(screen.getByRole('button', { name: 'More references' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'More references' }));
  expect(await screen.findByRole('link', { name: 'match-250' })).toBeVisible();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Previous references' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Previous references' }));
  expect(await screen.findByRole('link', { name: 'match-137' })).toBeVisible();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Previous references' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Previous references' }));
  expect(screen.getByText('No matches yet. Continue to check more references.')).toBeVisible();
  expect(requests.mock.calls.every(([offset]) => [137, 250].includes(offset))).toBe(true);
});
