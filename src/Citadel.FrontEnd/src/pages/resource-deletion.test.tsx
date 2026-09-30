import type {
  AuthorizedAction,
  BackupPolicyView,
  BackupRepositoryView,
  AuthorizedPool,
  AuthorizedProject,
} from '@/api/generated/api.types';
import { AutomationActionInfoActions } from '@/features/automation-actions/actions';
import { BackupPolicyInfoActions } from '@/features/backup-policies/actions';
import { BackupRepositoryInfoActions } from '@/features/backup-repositories/actions';
import { BuildPoolInfoActions } from '@/features/build-pools/actions';
import { BuildInfoActions } from '@/features/builds/actions';
import { useRead } from '@/lib/hooks';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';

const resource = { id: 'r1', name: 'Delete check' };
const cases = [
  {
    route: '/automation',
    api: 'automation/actions',
    read: 'getAutomationAction',
    list: 'listAutomationActions',
    action: <AutomationActionInfoActions.delete resource={resource as AuthorizedAction} />,
  },
  {
    route: '/backup-policies',
    api: 'backupPolicies',
    read: 'getBackupPolicy',
    list: 'listBackupPolicies',
    action: <BackupPolicyInfoActions.delete resource={resource as BackupPolicyView} />,
  },
  {
    route: '/backup-repositories',
    api: 'backupRepositories',
    read: 'getBackupRepository',
    list: 'listBackupRepositories',
    action: <BackupRepositoryInfoActions.delete resource={resource as BackupRepositoryView} />,
  },
  {
    route: '/build-pools',
    api: 'buildAgentPools',
    read: 'getBuildAgentPool',
    list: 'listBuildAgentPools',
    action: <BuildPoolInfoActions.delete resource={resource as AuthorizedPool} />,
  },
  {
    route: '/builds',
    api: 'buildProjects',
    read: 'getBuildProject',
    list: 'listBuildProjects',
    action: <BuildInfoActions.delete resource={resource as AuthorizedProject} />,
  },
] as const;

it.each(cases)('deleting from $route refreshes the list without rereading the removed resource', async (entry) => {
  let deleted = false;
  const read = vi.fn(() => (deleted ? new HttpResponse(null, { status: 404 }) : HttpResponse.json(resource)));
  const list = vi.fn(() => HttpResponse.json([]));
  server.use(
    http.get(`*/api/v1/${entry.api}/r1`, read),
    http.get(`*/api/v1/${entry.api}`, list),
    http.delete(`*/api/v1/${entry.api}/r1`, () => {
      deleted = true;
      return new HttpResponse(null, { status: 204 });
    }),
  );
  function Page() {
    const { data } = useRead(entry.read, { id: resource.id });
    useRead(entry.list);
    const location = useLocation();
    return (
      <>
        <output data-testid="location">{location.pathname}</output>
        {data && entry.action}
      </>
    );
  }
  // Keep the detail observer mounted during navigation to detect accidental invalidation.
  const { user } = renderCitadel(<Page />, { route: `${entry.route}/edit/r1/` });
  await user.click(await screen.findByRole('button', { name: 'Delete' }));
  const dialog = await screen.findByRole('dialog');
  await user.type(within(dialog).getByRole('textbox'), resource.name);
  await user.click(within(dialog).getByRole('button', { name: 'Delete' }));

  await waitFor(() => expect(screen.getByTestId('location').textContent).toBe(entry.route));
  await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
  expect(deleted).toBe(true);
  expect(read).toHaveBeenCalledOnce();
});
