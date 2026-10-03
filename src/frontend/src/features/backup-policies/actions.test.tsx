import type { BackupPolicyView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { BackupPolicyInfoActions } from './actions';

const backupPolicyId = '00000000-0000-0000-0000-000000000101';

const LocationProbe = () => {
  const location = useLocation();
  return <span data-testid="location">{location.pathname}</span>;
};

describe('BackupPolicyInfoActions', () => {
  it('returns to the policy list after archiving the open policy', async () => {
    server.use(
      http.delete(`http://localhost/api/v1/backupPolicies/${backupPolicyId}`, () =>
        HttpResponse.json({}, { status: 200 }),
      ),
    );
    const DeleteAction = BackupPolicyInfoActions.delete;
    const policy = { id: backupPolicyId, name: 'Daily backup' } as BackupPolicyView;
    const { user } = renderCitadel(
      <>
        <DeleteAction resource={policy} />
        <LocationProbe />
      </>,
      { route: `/backup-policies/edit/${backupPolicyId}` },
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = await screen.findByRole('dialog');
    await user.type(within(dialog).getByRole('textbox'), policy.name);
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));

    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/backup-policies'));
  });
});
