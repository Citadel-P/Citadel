import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { useBackupPolicyGroup } from './useBackupPolicyGroup';

function Probe() {
  const { data } = useBackupPolicyGroup('policy-1');
  return <output>{data?.data.latestRun?.status ?? 'Loading'}</output>;
}

it('updates the policy header from its subscribed snapshot without another GET', async () => {
  const policy = { id: 'policy-1', latestRun: { status: 'Running' } };
  const read = vi.fn(() => HttpResponse.json(policy));
  server.use(http.get('http://localhost/api/v1/backupPolicies/policy-1', read));
  const fake = new FakeRealtimeConnection();
  renderCitadel(<Probe />, {
    groups: {
      connectionFactory: () => fake.asRealtimeConnection(),
      startConnection: (connection) => connection.start(),
    },
  });
  expect(await screen.findByText('Running')).toBeVisible();
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', 'backup-policy:policy-1'));
  act(() => fake.emit('BackupPolicyInfoUpdated', { ...policy, latestRun: { status: 'Failed' } }, 'update'));
  expect(await screen.findByText('Failed')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
  act(() => fake.emit('BackupPolicyInfoUpdated', { id: 'other-policy', latestRun: { status: 'Succeeded' } }, 'update'));
  expect(screen.getByText('Failed')).toBeVisible();
});
