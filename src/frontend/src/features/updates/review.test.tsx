import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { toast } from 'sonner';
import {
  AgentEligibilityView,
  DatabaseInstallationView,
  type AgentUpdatesView,
  type UpdateStatusView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ReviewUpdates } from './review';

const status: UpdateStatusView = {
  instanceId: 'instance',
  installedVersion: '0.1.1',
  databaseVersion: '18.6',
  databaseAvailable: null,
  databaseManaged: false,
  databaseInstallation: DatabaseInstallationView.Unknown,
  databaseUpdateReason: null,
  channel: 'stable',
  checksAllowed: true,
  checksEnabled: true,
  checkError: null,
  installationUnavailableReason: null,
  lastAttempt: null,
  lastSuccess: null,
  operation: null,
  available: { version: '0.1.2', releaseUrl: 'https://example.test/release' },
};
const agents: AgentUpdatesView = {
  targets: ['A', 'B'].map((id) => ({
    id,
    name: `Agent ${id}`,
    kind: 'Platform',
    nodes: 1,
    supported: true,
    installedVersion: '0.1.0',
    availableVersion: '0.1.1',
    status: AgentEligibilityView.Available,
  })),
  schedule: { targets: [], enabled: false, cron: '0 3 * * *', timeZone: 'UTC', includeCore: false },
};

it('uses one entry point, reviews only the chosen component, and keeps Agent progress in the dialog', async () => {
  vi.spyOn(toast, 'success').mockImplementation(() => 'notice');
  const prepareCore = vi.fn();
  const prepareAgents = vi.fn();
  const requests: unknown[] = [];
  const operation = { id: 'operation', finished: false, targets: [{ target: agents.targets[1], phase: 'Updating' }] };
  server.use(
    http.post('http://localhost/api/v1/updates/review', () => {
      prepareCore();
      return HttpResponse.json({});
    }),
    http.post('http://localhost/api/v1/updates/agents/review', async ({ request }) => {
      prepareAgents(await request.json());
      return HttpResponse.json({ id: 'review', targets: [agents.targets[1]] });
    }),
    http.post('http://localhost/api/v1/updates/agents/approve', async ({ request }) => {
      requests.push(await request.json());
      return requests.length === 1 ? new HttpResponse(null, { status: 503 }) : HttpResponse.json(operation);
    }),
    http.get('http://localhost/api/v1/updates/agents/operations/operation', () => HttpResponse.json(operation)),
  );
  const { user, queryClient } = renderCitadel(<ReviewUpdates status={status} agents={agents} selected={['B']} />);
  expect(screen.getAllByRole('button')).toHaveLength(1);
  await user.click(screen.getByRole('button', { name: 'Review and update' }));
  const dialog = await screen.findByRole('dialog');
  expect(prepareCore).not.toHaveBeenCalled();
  expect(prepareAgents).not.toHaveBeenCalled();
  await user.click(within(dialog).getByRole('button', { name: '1 Agent update' }));
  expect(await within(dialog).findByText('Agent B')).toBeVisible();
  expect(prepareAgents).toHaveBeenCalledExactlyOnceWith({ targets: ['B'] });
  await user.click(within(dialog).getByRole('button', { name: 'Update now' }));
  expect(await within(dialog).findByRole('alert')).toHaveTextContent('Approval could not be confirmed');
  await user.click(within(dialog).getByRole('button', { name: 'Update now' }));
  expect(await within(dialog).findByText('Updating')).toBeVisible();
  expect(within(dialog).getByText('Updating').querySelector('.animate-spin')).toBeInTheDocument();
  expect(requests).toHaveLength(2);
  expect(requests[0]).toEqual(requests[1]);
  expect(prepareCore).not.toHaveBeenCalled();
  expect(screen.getAllByRole('dialog')).toHaveLength(1);
  act(() =>
    queryClient.setQueryData(['getAgentUpdateOperation', { id: 'operation' }], {
      data: {
        ...operation,
        finished: true,
        targets: [{ ...operation.targets[0], phase: 'Succeeded' }],
      },
    }),
  );
  expect(await within(dialog).findByText('Update results')).toBeVisible();
  expect(within(dialog).getByText('Succeeded')).toHaveAttribute('data-slot', 'badge');
  await waitFor(() => expect(toast.success).toHaveBeenCalledWith('Agent updates finished'));
});

it.each(['Succeeded', 'Failed'])('notifies %s once after the progress dialog is closed', async (phase) => {
  const success = vi.spyOn(toast, 'success').mockImplementation(() => 'notice');
  const error = vi.spyOn(toast, 'error').mockImplementation(() => 'notice');
  const operation = { id: 'operation', finished: false, targets: [{ target: agents.targets[0], phase: 'Updating' }] };
  server.use(
    http.get('http://localhost/api/v1/updates/agents/operations/operation', () => HttpResponse.json(operation)),
  );
  const { user, queryClient } = renderCitadel(
    <ReviewUpdates status={status} agents={{ ...agents, operation } as AgentUpdatesView} />,
  );
  await user.click(screen.getByRole('button', { name: 'View progress' }));
  await screen.findByText('Updating');
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  const finished = { data: { ...operation, finished: true, targets: [{ ...operation.targets[0], phase }] } };
  act(() => {
    queryClient.setQueryData(['getAgentUpdateOperation', { id: 'operation' }], finished);
  });
  await waitFor(() => expect(phase === 'Failed' ? error : success).toHaveBeenCalledTimes(1));
  act(() => {
    queryClient.setQueryData(['getAgentUpdateOperation', { id: 'operation' }], { ...finished });
  });
  expect(phase === 'Failed' ? error : success).toHaveBeenCalledTimes(1);
});

it('reviews a database-only upgrade through the existing confirmation dialog', async () => {
  const review = vi.fn();
  server.use(
    http.post('http://localhost/api/v1/updates/review', async ({ request }) => {
      review(await request.json());
      return HttpResponse.json({
        id: 'database-review',
        component: 'Database',
        installedVersion: '17.11',
        targetVersion: '18.6',
        expiresAt: '2099-01-01T00:00:00Z',
      });
    }),
  );
  const { user } = renderCitadel(
    <ReviewUpdates
      status={{
        ...status,
        available: null,
        databaseVersion: '17.11',
        databaseManaged: true,
        databaseAvailable: '18.6',
      }}
    />,
  );
  await user.click(screen.getByRole('button', { name: 'Review and update' }));
  const dialog = await screen.findByRole('dialog');
  expect(await within(dialog).findByText('17.11')).toBeVisible();
  expect(within(dialog).getByRole('heading', { name: 'Update PostgreSQL' })).toBeVisible();
  expect(review).toHaveBeenCalledExactlyOnceWith({ version: '18.6', database: true });
  expect(within(dialog).getByRole('button', { name: 'Update and restart' })).toBeEnabled();
});
