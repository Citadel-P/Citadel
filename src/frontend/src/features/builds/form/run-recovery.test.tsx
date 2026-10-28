import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import type { BuildRunView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BuildRunRecovery } from './run-recovery';

const run = { id: 'run-1', buildProjectId: 'project-1', platformSnapshot: { name: 'Builder' } } as BuildRunView;
const reservation = { attemptId: 'attempt-1', daemonId: 'docker-1', authorityId: 'old-agent' };

function setup(administrator: boolean) {
  server.use(
    http.get('http://localhost/api/v1/profile', () =>
      HttpResponse.json({ authorization: { isAdministrator: administrator } }),
    ),
    http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
  );
}

it('requires notes and acknowledgment, then releases the selected attempt and refreshes the run', async () => {
  setup(true);
  let resolved = false;
  const post = vi.fn(async ({ request }: { request: Request }) => {
    expect(await request.json()).toEqual({
      attemptId: reservation.attemptId,
      reason: 'Checked host and stopped old push',
      acknowledged: true,
    });
    resolved = true;
    return new HttpResponse(null, { status: 204 });
  });
  server.use(
    http.get('http://localhost/api/v1/buildRuns/run-1/recovery', () =>
      HttpResponse.json({ reservation: resolved ? null : reservation }),
    ),
    http.post('http://localhost/api/v1/buildRuns/run-1/recovery', post),
  );
  const { user, queryClient } = renderCitadel(<BuildRunRecovery run={run} />);
  const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
  await user.click(await screen.findByRole('button', { name: 'Resolve cleanup' }));
  const release = screen.getByRole('button', { name: 'Release capacity' });
  expect(release).toBeDisabled();
  await user.type(screen.getByLabelText('Resolution notes'), 'Checked host and stopped old push');
  expect(release).toBeDisabled();
  await user.click(screen.getByRole('checkbox'));
  expect(release).toBeEnabled();
  await user.click(release);
  await waitFor(() => expect(post).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getBuildRun', { id: run.id }] });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getBuildRunLogs', { id: run.id }] });
});

it('does not read recovery details or expose resolution to non-administrators', async () => {
  setup(false);
  const read = vi.fn(() => HttpResponse.json({ reservation }));
  server.use(http.get('http://localhost/api/v1/buildRuns/run-1/recovery', read));
  const { queryClient } = renderCitadel(<BuildRunRecovery run={run} />);
  await waitFor(() => expect(queryClient.getQueryState(['getCurrentProfile', {}])?.status).toBe('success'));
  expect(read).not.toHaveBeenCalled();
  expect(screen.queryByRole('button', { name: 'Resolve cleanup' })).not.toBeInTheDocument();
});

it('keeps acknowledgment open when the server refuses recovery', async () => {
  setup(true);
  server.use(
    http.get('http://localhost/api/v1/buildRuns/run-1/recovery', () => HttpResponse.json({ reservation })),
    http.post('http://localhost/api/v1/buildRuns/run-1/recovery', () =>
      HttpResponse.json({ message: 'Restart the original Agent first.' }, { status: 409 }),
    ),
  );
  const { user } = renderCitadel(<BuildRunRecovery run={run} />);
  await user.click(await screen.findByRole('button', { name: 'Resolve cleanup' }));
  await user.type(screen.getByLabelText('Resolution notes'), 'Checked');
  await user.click(screen.getByRole('checkbox'));
  await user.click(screen.getByRole('button', { name: 'Release capacity' }));
  expect(await screen.findByRole('alert')).toBeVisible();
  expect(screen.getByRole('dialog')).toBeVisible();
  expect(screen.getByLabelText('Resolution notes')).toHaveValue('Checked');
});
