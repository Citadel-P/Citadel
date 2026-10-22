import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { toast } from 'sonner';
import {
  BackupRepositoryView,
  BackupRepositoryType,
  BackupRepositoryStatus,
  ResourceControlState,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useBackupRepositoriesGroup } from '../backup-repositories/hooks/useBackupRepositoriesGroup';
import { toUpdateInput } from '../backup-repositories/form/form';
import { RepositoryCredentials } from '../backup-repositories/credentials';
import { BackupRepositoriesSection } from './repositories';

vi.mock('../backup-repositories/hooks/useBackupRepositoriesGroup', () => ({ useBackupRepositoriesGroup: vi.fn() }));
const repository = {
  archivedAt: null,
  controlStartedAt: null,
  currentRunId: null,
  createdAt: '2026-10-21T12:00:00Z',
  updatedAt: '2026-10-21T12:00:00Z',
  createdByActorId: 'admin',
  lastCheckedAt: null,
  lastPrunedAt: null,
  normalizedName: 'PRODUCTION BACKUPS',
  rowVersion: 0,
  id: 'repository',
  name: 'Production backups',
  type: BackupRepositoryType.S3Compatible,
  status: BackupRepositoryStatus.Ready,
  controlState: ResourceControlState.Idle,
  passwordSecretId: 'password',
  capabilities: { canRead: true, canWrite: true, canExecute: true },
  spec: {
    $type: BackupRepositoryType.S3Compatible,
    endpoint: 'https://storage.example.test',
    bucket: 'backups',
    prefix: 'production',
    accessKeySecretId: 'access',
    secretKeySecretId: 'secret',
  },
} satisfies BackupRepositoryView;

beforeEach(() => {
  vi.mocked(useBackupRepositoriesGroup).mockReturnValue({
    repositories: [repository],
    isLoading: false,
    capabilities: repository.capabilities,
  } as ReturnType<typeof useBackupRepositoriesGroup>);
  server.use(
    http.get('http://localhost/api/v1/resourceBindings/secrets', () =>
      HttpResponse.json({
        secrets: [
          { id: 'password', name: 'Restic password' },
          { id: 'access', name: 'Current access' },
          { id: 'secret', name: 'Current secret' },
          { id: 'replacement', name: 'Replacement key' },
          { id: 'rotated', name: 'Rotated key' },
        ],
      }),
    ),
  );
});

it('edits only metadata on a ready repository and keeps storage readable', async () => {
  const patches: unknown[] = [];
  server.use(
    http.patch('http://localhost/api/v1/backupRepositories/repository', async ({ request }) => {
      patches.push(await request.json());
      return HttpResponse.json(repository);
    }),
  );
  const { user } = renderCitadel(<BackupRepositoriesSection />);
  await user.click(screen.getByText('Production backups'));
  const dialog = within(screen.getByRole('dialog'));
  expect(await dialog.findByText('Restic password')).toBeVisible();
  expect(dialog.getByText('https://storage.example.test')).toBeVisible();
  expect(dialog.queryByRole('combobox')).not.toBeInTheDocument();
  const save = dialog.getByRole('button', { name: 'Save changes' });
  expect(save).toBeDisabled();
  const name = dialog.getByRole('textbox', { name: 'Name' });
  await user.clear(name);
  await user.type(name, 'Renamed backups');
  expect(dialog.queryByRole('textbox', { name: /Description/ })).not.toBeInTheDocument();
  await user.click(dialog.getByText('Maintenance', { exact: true }));
  expect(dialog.getByRole('button', { name: /Validate/ })).toBeDisabled();
  await user.click(save);
  await vi.waitFor(() => expect(patches).toEqual([{ name: 'Renamed backups' }]));
  await vi.waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});

it('disables saving when an edit is reverted and retains input after a conflict', async () => {
  server.use(
    http.patch('http://localhost/api/v1/backupRepositories/repository', () =>
      HttpResponse.json({ title: 'Conflict', detail: 'Name already exists', status: 409 }, { status: 409 }),
    ),
  );
  const { user } = renderCitadel(<BackupRepositoriesSection />);
  await user.click(screen.getByText('Production backups'));
  const dialog = within(screen.getByRole('dialog'));
  const name = dialog.getByRole('textbox', { name: 'Name' });
  await user.clear(name);
  await user.type(name, 'Production backups');
  expect(dialog.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  await user.type(name, ' duplicate');
  await user.click(dialog.getByRole('button', { name: 'Save changes' }));
  await vi.waitFor(() => expect(dialog.getByRole('button', { name: 'Save changes' })).toBeEnabled());
  expect(name).toHaveValue('Production backups duplicate');
});

it('explains read-only access and hides saving', async () => {
  vi.mocked(useBackupRepositoriesGroup).mockReturnValue({
    repositories: [{ ...repository, capabilities: { ...repository.capabilities, canWrite: false } }],
    isLoading: false,
    capabilities: { canWrite: false },
  } as ReturnType<typeof useBackupRepositoriesGroup>);
  const { user } = renderCitadel(<BackupRepositoriesSection />);
  await user.click(screen.getByText('Production backups'));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByRole('status')).toHaveTextContent('Write permission is required');
  expect(dialog.getByRole('textbox', { name: 'Name' })).toBeDisabled();
  expect(dialog.queryByRole('textbox', { name: /Description/ })).not.toBeInTheDocument();
  expect(dialog.queryByRole('button', { name: 'Save changes' })).not.toBeInTheDocument();
  await user.click(dialog.getByText('Maintenance', { exact: true }));
  expect(dialog.getByRole('button', { name: /Validate/ })).toBeDisabled();
});

it('omits unchanged normalized fields from patches', () => {
  expect(
    toUpdateInput(
      { ...repository, name: ' Production backups ', spec: { ...repository.spec, prefix: 'production', region: null } },
      repository,
    ),
  ).toEqual({});
});

async function openCredentialEditor() {
  const view = renderCitadel(<BackupRepositoriesSection />);
  await view.user.click(screen.getByText('Production backups'));
  const dialog = within(screen.getByRole('dialog'));
  await view.user.click(dialog.getByText('Update S3 credentials', { exact: true }));
  const credentials = within(dialog.getByRole('group', { name: 'Replacement S3 credentials' }));
  await vi.waitFor(() => expect(credentials.getAllByRole('combobox')[0]).toBeEnabled());
  return { ...view, dialog, credentials };
}

it('validates replacement credentials separately from repository metadata', async () => {
  const requests: unknown[] = [];
  server.use(
    http.post('http://localhost/api/v1/backupRepositories/repository/credentials', async ({ request }) => {
      requests.push(await request.json());
      return HttpResponse.json({ ...repository, spec: { ...repository.spec, accessKeySecretId: 'replacement' } });
    }),
  );
  const { user, dialog, credentials } = await openCredentialEditor();
  expect(dialog.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
  await user.click(credentials.getAllByRole('combobox')[0]);
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  expect(dialog.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  await user.click(dialog.getByRole('button', { name: 'Validate and update' }));
  await vi.waitFor(() =>
    expect(requests).toEqual([
      {
        accessKeySecretId: 'replacement',
        secretKeySecretId: 'secret',
        sessionTokenSecretId: null,
        location: 'Core',
        platformId: null,
      },
    ]),
  );
  await vi.waitFor(() => expect(dialog.queryByRole('button', { name: 'Validate and update' })).not.toBeInTheDocument());
});

it('keeps rejected credential selections for retry without closing the editor', async () => {
  const notify = vi.spyOn(toast, 'error');
  server.use(
    http.post('http://localhost/api/v1/backupRepositories/repository/credentials', () =>
      HttpResponse.json(
        { title: 'Validation failed', detail: 'Existing credentials were not changed.', status: 400 },
        { status: 400 },
      ),
    ),
  );
  const { user, dialog, credentials } = await openCredentialEditor();
  await user.click(credentials.getAllByRole('combobox')[0]);
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  await user.click(dialog.getByRole('button', { name: 'Validate and update' }));
  expect(await dialog.findByRole('alert')).toHaveTextContent('Existing credentials were not changed.');
  expect(notify).toHaveBeenCalledWith('Request failed', { description: 'Existing credentials were not changed.' });
  notify.mockRestore();
  await vi.waitFor(() => expect(dialog.getByRole('button', { name: 'Validate and update' })).toBeEnabled());
  expect(credentials.getAllByRole('combobox')[0]).toHaveTextContent('Replacement key');
  expect(dialog.getByRole('textbox', { name: 'Name' })).toHaveValue(repository.name);
  await user.click(dialog.getByRole('button', { name: 'Reset selections' }));
  expect(dialog.queryByRole('alert')).not.toBeInTheDocument();
  expect(credentials.getAllByRole('combobox')[0]).toHaveTextContent('Current access');
  expect(dialog.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
});

it.each([false, true])('refreshes untouched credential selections when the editor is open: %s', async (open) => {
  const { user, rerender } = renderCitadel(<RepositoryCredentials repository={repository} />);
  if (open) await user.click(screen.getByText('Update S3 credentials', { exact: true }));
  rerender(
    <RepositoryCredentials
      repository={{ ...repository, spec: { ...repository.spec, accessKeySecretId: 'rotated' } }}
    />,
  );
  if (!open) await user.click(screen.getByText('Update S3 credentials', { exact: true }));
  const access = within(screen.getByRole('group', { name: 'Access key secret' }));
  await vi.waitFor(() => expect(access.getByRole('combobox')).toHaveTextContent('Rotated key'));
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});

it('preserves edits on metadata refresh and requires a reset after a concurrent credential change', async () => {
  const { user, rerender } = renderCitadel(<RepositoryCredentials repository={repository} />);
  await user.click(screen.getByText('Update S3 credentials', { exact: true }));
  const access = within(screen.getByRole('group', { name: 'Access key secret' }));
  await vi.waitFor(() => expect(access.getByRole('combobox')).toBeEnabled());
  await user.click(access.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  rerender(<RepositoryCredentials repository={{ ...repository, name: 'Renamed' }} />);
  expect(access.getByRole('combobox')).toHaveTextContent('Replacement key');
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeEnabled();

  rerender(
    <RepositoryCredentials
      repository={{ ...repository, spec: { ...repository.spec, accessKeySecretId: 'rotated' } }}
    />,
  );
  expect(screen.getByRole('alert')).toHaveTextContent('Credentials changed while you were editing');
  expect(access.getByRole('combobox')).toHaveTextContent('Replacement key');
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Reset selections' }));
  expect(access.getByRole('combobox')).toHaveTextContent('Rotated key');
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  await user.click(access.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeEnabled();
});

it('does not treat refreshed credentials as unsaved repository metadata', async () => {
  const { user, dialog, rerender } = await openCredentialEditor();
  vi.mocked(useBackupRepositoriesGroup).mockReturnValue({
    repositories: [{ ...repository, spec: { ...repository.spec, accessKeySecretId: 'rotated' } }],
    isLoading: false,
    capabilities: repository.capabilities,
  } as ReturnType<typeof useBackupRepositoriesGroup>);
  rerender(<BackupRepositoriesSection />);
  expect(dialog.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  const access = within(dialog.getByRole('group', { name: 'Access key secret' }));
  expect(access.getByRole('combobox')).toHaveTextContent('Rotated key');
  await user.click(access.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  expect(dialog.getByRole('button', { name: 'Validate and update' })).toBeEnabled();
});

it('allows a new edit when refreshed credentials already match the previous selection', async () => {
  const { user, rerender } = renderCitadel(<RepositoryCredentials repository={repository} />);
  await user.click(screen.getByText('Update S3 credentials', { exact: true }));
  const access = within(screen.getByRole('group', { name: 'Access key secret' }));
  await vi.waitFor(() => expect(access.getByRole('combobox')).toBeEnabled());
  await user.click(access.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Replacement key' }));
  rerender(
    <RepositoryCredentials
      repository={{ ...repository, spec: { ...repository.spec, accessKeySecretId: 'replacement' } }}
    />,
  );
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
  await user.click(access.getByRole('combobox'));
  await user.click(screen.getByRole('option', { name: 'Rotated key' }));
  expect(screen.getByRole('button', { name: 'Validate and update' })).toBeEnabled();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});

it('requires Execute as well as Write to update credentials', async () => {
  vi.mocked(useBackupRepositoriesGroup).mockReturnValue({
    repositories: [{ ...repository, capabilities: { canRead: true, canWrite: true, canExecute: false } }],
    isLoading: false,
    capabilities: repository.capabilities,
  } as ReturnType<typeof useBackupRepositoriesGroup>);
  const { user } = renderCitadel(<BackupRepositoriesSection />);
  await user.click(screen.getByText('Production backups'));
  const dialog = within(screen.getByRole('dialog'));
  await user.click(dialog.getByText('Update S3 credentials', { exact: true }));
  expect(dialog.getByText('Write and Execute permissions are required.')).toBeVisible();
  expect(dialog.getByRole('button', { name: 'Validate and update' })).toBeDisabled();
  expect(dialog.getByRole('textbox', { name: 'Name' })).toBeEnabled();
});
