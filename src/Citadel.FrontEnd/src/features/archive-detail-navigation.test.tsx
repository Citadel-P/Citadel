import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import type { BackupPolicyView, BuildProjectView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { BackupPolicyInfoActions } from './backup-policies/actions';
import { BuildInfoActions } from './builds/actions';

const backupPolicyId = '00000000-0000-0000-0000-000000000101';
const buildProjectId = '00000000-0000-0000-0000-000000000201';

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{location.pathname}</span>;
}

describe('detail archive navigation', () => {
  it('returns to the backup policy list after archiving the open policy', async () => {
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

    await confirmDelete(user, 'Daily backup');

    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/backup-policies'));
  });

  it('returns to the build list after archiving the open build', async () => {
    server.use(
      http.delete(`http://localhost/api/v1/buildProjects/${buildProjectId}`, () =>
        HttpResponse.json({}, { status: 200 }),
      ),
    );
    const DeleteAction = BuildInfoActions.delete;
    const project = { id: buildProjectId, name: 'Application image' } as BuildProjectView;
    const { user } = renderCitadel(
      <>
        <DeleteAction resource={project} />
        <LocationProbe />
      </>,
      { route: `/builds/edit/${buildProjectId}` },
    );

    await confirmDelete(user, 'Application image');

    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/builds'));
  });
});

async function confirmDelete(user: ReturnType<typeof renderCitadel>['user'], resourceName: string) {
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = await screen.findByRole('dialog');
  await user.type(within(dialog).getByRole('textbox'), resourceName);
  await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
}
