import { BackupRepositoryStatus, type BackupRepositoryView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { BackupRepositoryInfoActions } from './actions';

const repositoryId = '00000000-0000-0000-0000-000000000102';

describe('Backup repository initialization', () => {
  it('disables Initialize for a ready repository while keeping Check available', async () => {
    const initialize = vi.fn();
    server.use(http.post(`http://localhost/api/v1/backupRepositories/${repositoryId}/initialize`, initialize));
    const Initialize = BackupRepositoryInfoActions.initialize;
    const Check = BackupRepositoryInfoActions.check;
    const repository = {
      id: repositoryId,
      status: BackupRepositoryStatus.Ready,
      capabilities: { canExecute: true },
    } as BackupRepositoryView;
    const { user } = renderCitadel(
      <>
        <Initialize resource={repository} />
        <Check resource={repository} />
      </>,
    );

    const button = screen.getByRole('button', { name: 'Initialize' });
    expect(button).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Check' })).toBeEnabled();
    await user.click(button);
    expect(initialize).not.toHaveBeenCalled();
  });

  it('initializes a new repository using its execution location', async () => {
    const initialize = vi.fn(() => new HttpResponse(null, { status: 204 }));
    server.use(
      http.post(`http://localhost/api/v1/backupRepositories/${repositoryId}/initialize`, async ({ request }) => {
        expect(await request.json()).toEqual({ location: 'Core', platformId: null });
        return initialize();
      }),
    );
    const Initialize = BackupRepositoryInfoActions.initialize;
    const repository = {
      id: repositoryId,
      status: BackupRepositoryStatus.Unknown,
      spec: { $type: 'FileSystem', location: 'Core', path: '/backups' },
      capabilities: { canExecute: true },
    } as BackupRepositoryView;
    const { user } = renderCitadel(<Initialize resource={repository} />);

    await user.click(screen.getByRole('button', { name: 'Initialize' }));
    await waitFor(() => expect(initialize).toHaveBeenCalledOnce());
  });
});
