import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  ScriptRunStatus,
  ScriptRunTrigger,
  type AuthorizedScript,
  type ScriptRunView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { ScriptRunsTab } from './runs';

const scriptId = '00000000-0000-0000-0000-000000000201';
const initiator = '00000000-0000-0000-0000-000000000301';
const runAs = '00000000-0000-0000-0000-000000000302';
const run: ScriptRunView = {
  id: '00000000-0000-0000-0000-000000000202',
  scriptId,
  scriptName: 'Prune images',
  executorId: null,
  attemptId: null,
  cleanupPending: false,
  trigger: ScriptRunTrigger.Test,
  status: ScriptRunStatus.Succeeded,
  runAsActorId: runAs,
  triggeredByActorId: initiator,
  argsJson: '{}',
  codeSnapshot: null,
  codeHash: '',
  timeoutSeconds: 300,
  queuedAt: '2026-10-29T10:00:00Z',
  startedAt: '2026-10-29T10:00:01Z',
  finishedAt: '2026-10-29T10:00:04Z',
  durationMs: 3000,
  exitCode: 0,
  logs: null,
  errorMessage: null,
};

function setup(runs = [run], administrator = true) {
  const list = vi.fn(() => HttpResponse.json({ runs }));
  const actors = vi.fn(({ params }: { params: { id?: string | readonly string[] } }) =>
    HttpResponse.json({ id: params.id, type: 'User', name: params.id === initiator ? 'admin' : 'automation' }),
  );
  server.use(
    http.get(`http://localhost/api/v1/scripts/${scriptId}/runs`, list),
    http.get(`http://localhost/api/v1/scripts/${scriptId}/runs/:runId/logs`, () =>
      HttpResponse.json({ logs: 'Pruned 34 images.' }),
    ),
    http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
    http.get('http://localhost/api/v1/profile', () =>
      HttpResponse.json({ authorization: { isAdministrator: administrator } }),
    ),
    http.get('http://localhost/api/v1/actors/:id', actors),
  );
  const fake = new FakeRealtimeConnection();
  const view = renderCitadel(<ScriptRunsTab resource={{ id: scriptId } as AuthorizedScript} />, {
    groups: { connectionFactory: () => fake, startConnection: (connection) => connection.start() },
  });
  return { ...view, fake, list, actors };
}

it('groups run and result details and opens metadata from the colored play control', async () => {
  const { user, actors } = setup([
    { ...run, runAsActorId: initiator },
    { ...run, id: 'failed-run', status: ScriptRunStatus.Failed, exitCode: 1 },
  ]);
  const button = await screen.findByRole('button', { name: 'View succeeded run details' });
  const row = button.closest('tr')!;
  expect(screen.getAllByRole('columnheader')).toHaveLength(4);
  expect(within(row).getByText('Test run')).toBeVisible();
  expect(within(row).getByText('3.0 s')).toBeVisible();
  expect(within(row).getByText('Exit code 0')).toBeVisible();
  expect(within(button).getByRole('img', { name: 'Run: Succeeded' }).firstElementChild).toHaveClass('text-green-500');
  expect(screen.getByRole('img', { name: 'Run: Failed' }).firstElementChild).toHaveClass('text-red-500');
  expect(actors).not.toHaveBeenCalled();

  button.focus();
  await user.keyboard('{Enter}');
  const sheet = await screen.findByRole('dialog');
  expect(within(sheet).getByRole('heading', { name: 'Prune images logs' })).toBeVisible();
  expect(await within(sheet).findByText('admin')).toBeVisible();
  for (const label of ['Triggered by', 'Queued', 'Duration']) {
    expect(within(sheet).getByText(label)).toBeVisible();
  }
  for (const label of ['Run as', 'Status', 'Started', 'Finished', 'Exit code']) {
    expect(within(sheet).queryByText(label, { exact: true })).not.toBeInTheDocument();
  }
  expect(within(sheet).getByText('3.0 s')).toBeVisible();
  expect(await within(sheet).findByText('Pruned 34 images.')).toBeVisible();
  expect(within(sheet).queryByRole('button', { name: 'Timestamps' })).not.toBeInTheDocument();
  expect(within(sheet).queryByRole('button', { name: 'Wrap Lines' })).not.toBeInTheDocument();
});

it.each([ScriptRunTrigger.Schedule, ScriptRunTrigger.Webhook])(
  'distinguishes an automatic %s trigger from the account used to run it',
  async (trigger) => {
    const { user, actors } = setup([{ ...run, trigger, triggeredByActorId: null }]);
    await user.click(await screen.findByRole('button', { name: 'View succeeded run details' }));
    const sheet = await screen.findByRole('dialog');
    expect(within(sheet).getByText(trigger, { exact: true })).toBeVisible();
    expect(await within(sheet).findByText('automation')).toBeVisible();
    expect(actors).toHaveBeenCalledTimes(1);
  },
);

it('shows stored actor identifiers without requesting administrator-only actor data for other viewers', async () => {
  const { user, actors } = setup([run], false);
  await user.click(await screen.findByRole('button', { name: 'View succeeded run details' }));
  const sheet = await screen.findByRole('dialog');
  expect(within(sheet).getByTitle(initiator)).toHaveTextContent('Actor 00000000');
  expect(within(sheet).getByTitle(runAs)).toHaveTextContent('Actor 00000000');
  expect(await within(sheet).findByText('Pruned 34 images.')).toBeVisible();
  expect(actors).not.toHaveBeenCalled();
});

it('keeps cancellation and live result updates while showing failure details in the open sheet', async () => {
  const active = { ...run, status: ScriptRunStatus.Running, finishedAt: null, durationMs: null, exitCode: null };
  const cancel = vi.fn(() => new HttpResponse(null, { status: 204 }));
  server.use(http.post(`http://localhost/api/v1/scripts/${scriptId}/runs/${run.id}/cancel`, cancel));
  const { user, fake, list } = setup([active]);
  await user.click(await screen.findByRole('button', { name: 'Cancel run' }));
  await waitFor(() => expect(cancel).toHaveBeenCalledTimes(1));
  await user.click(screen.getByRole('button', { name: 'View running run details' }));
  const sheet = await screen.findByRole('dialog');
  expect(within(sheet).getByText('In progress')).toBeVisible();
  await waitFor(() => expect(fake.listenerCount('ScriptRunInfoUpdated')).toBe(1));
  act(() =>
    fake.emit(
      'ScriptRunInfoUpdated',
      { ...run, status: ScriptRunStatus.TimedOut, exitCode: 124, errorMessage: 'Execution timed out.' },
      'update',
    ),
  );
  expect(await within(sheet).findByText('Test run · Timed out')).toBeVisible();
  expect(within(sheet).getByText('3.0 s')).toBeVisible();
  expect(within(sheet).getByText('Execution timed out.')).toBeVisible();
  expect(within(sheet).getByText('Exit code')).toBeVisible();
  expect(within(sheet).getByText('124')).toBeVisible();
  expect(list).toHaveBeenCalledTimes(1);
});

it('distinguishes a queued run from a completed zero-duration run', async () => {
  const { user } = setup([
    { ...run, status: ScriptRunStatus.Queued, startedAt: null, finishedAt: null, durationMs: null, exitCode: null },
    { ...run, id: 'instant-run', durationMs: 0 },
  ]);
  const button = await screen.findByRole('button', { name: 'View queued run details' });
  expect(screen.getByText('Not started')).toBeVisible();
  expect(screen.getByText('0 ms')).toBeVisible();
  await user.click(button);
  const sheet = await screen.findByRole('dialog');
  expect(within(sheet).getByText('Not started')).toBeVisible();
  expect(within(sheet).queryByText('Started', { exact: true })).not.toBeInTheDocument();
  expect(within(sheet).queryByText('Finished', { exact: true })).not.toBeInTheDocument();
});
