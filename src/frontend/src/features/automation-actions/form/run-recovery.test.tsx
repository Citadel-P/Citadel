import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import type { AutomationRunView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { AutomationRunRecovery } from './run-recovery';

const run = {
  id: 'run-1',
  actionId: 'action-1',
  attemptId: 'attempt-1',
  cleanupPending: true,
  finishedAt: '2026-10-27T00:00:00Z',
} as AutomationRunView;
function setup(administrator: boolean) {
  server.use(
    http.get('http://localhost/api/v1/profile', () =>
      HttpResponse.json({ authorization: { isAdministrator: administrator } }),
    ),
    http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
  );
}
it.each([false, true])(
  'requires acknowledgment without mutation-driven refetches (connected: %s)',
  async (connected) => {
    setup(true);
    const post = vi.fn(async ({ request }: { request: Request }) => {
      expect(await request.json()).toEqual({
        attemptId: 'attempt-1',
        reason: 'Old script stopped',
        acknowledged: true,
      });
      return new HttpResponse(null, { status: 204 });
    });
    server.use(http.post('http://localhost/api/v1/automation/actions/action-1/runs/run-1/recovery', post));
    const fake = new FakeRealtimeConnection();
    const { user, queryClient } = renderCitadel(
      <AutomationRunRecovery run={run} />,
      connected
        ? {
            groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
          }
        : {},
    );
    if (connected) await waitFor(() => expect(fake.start).toHaveBeenCalledTimes(1));
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
    await user.click(await screen.findByRole('button', { name: 'Resolve cleanup' }));
    const release = screen.getByRole('button', { name: 'Release capacity' });
    expect(release).toBeDisabled();
    await user.type(screen.getByLabelText('Resolution notes'), 'Old script stopped');
    expect(release).toBeDisabled();
    await user.click(screen.getByRole('checkbox'));
    await user.click(release);
    await waitFor(() => expect(post).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(invalidate).not.toHaveBeenCalled();
  },
);
it('shows pending cleanup but no recovery action to non-administrators', async () => {
  setup(false);
  const { queryClient } = renderCitadel(<AutomationRunRecovery run={run} />);
  await waitFor(() => expect(queryClient.getQueryState(['getCurrentProfile', {}])?.status).toBe('success'));
  expect(screen.getByText('Cleanup pending')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Resolve cleanup' })).not.toBeInTheDocument();
});
