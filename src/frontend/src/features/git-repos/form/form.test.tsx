import { Route, Routes } from 'react-router';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { GitRepositorySyncMode, type GitRepositoryConfigResponse } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { GitRepoForm } from './form';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value, readOnly }: { value: string; readOnly: boolean }) => (
    <textarea aria-label="Saved commands" value={value} readOnly={readOnly} />
  ),
  MonacoToArrayEditor: ({ value, onChange }: { value?: string[]; onChange: (value: string[]) => void }) => (
    <textarea
      aria-label="Commands"
      value={(value ?? []).join('\n')}
      onChange={(event) => onChange(event.target.value.split('\n').filter(Boolean))}
    />
  ),
  MonacoDiff: () => null,
}));

it.each([false, true])('honors write permission for hook controls (disabled=%s)', async (disabled) => {
  const resource: GitRepositoryConfigResponse = {
    id: 'repo-1',
    name: 'source',
    description: null,
    url: 'https://example.test/app.git',
    defaultBranch: 'main',
    gitAccountId: null,
    tags: [],
    webhook: null,
    syncMode: GitRepositorySyncMode.Manual,
    syncIntervalMinutes: null,
    onClone: { path: 'setup', commands: ['echo clone'] },
    onPull: { path: 'scripts', commands: ['echo pull'] },
  };
  let payload: any;
  server.use(
    http.get('http://localhost/api/v1/gitRepositories/repo-1/_cfg', () => HttpResponse.json(resource)),
    http.get('http://localhost/api/v1/gitAccounts', () => HttpResponse.json({ gitAccounts: [], capabilities: {} })),
    http.patch('http://localhost/api/v1/gitRepositories/repo-1', async ({ request }) => {
      payload = await request.json();
      return HttpResponse.json({});
    }),
  );
  const { user } = renderCitadel(
    <Routes>
      <Route path="/git-repos/edit/:id" element={<GitRepoForm mode="edit" disabled={disabled} />} />
    </Routes>,
    { route: '/git-repos/edit/repo-1' },
  );
  await screen.findByDisplayValue('setup');
  if (disabled) {
    expect(screen.getByDisplayValue('setup')).toBeDisabled();
    expect(screen.getByDisplayValue('scripts')).toBeDisabled();
    expect(screen.getByDisplayValue('echo clone')).toHaveAttribute('readonly');
    expect(screen.getByDisplayValue('echo pull')).toHaveAttribute('readonly');
    return;
  }
  expect(screen.getByDisplayValue('setup')).toBeEnabled();
  expect(screen.getByDisplayValue('scripts')).toBeEnabled();
  expect(screen.getByDisplayValue('echo clone')).not.toHaveAttribute('readonly');
  expect(screen.getByDisplayValue('echo pull')).not.toHaveAttribute('readonly');
  await user.clear(screen.getByRole('textbox', { name: 'Default Branch' }));
  await user.type(screen.getByRole('textbox', { name: 'Default Branch' }), 'release');
  const save = screen.getAllByRole('button', { name: 'Save' })[0];
  await waitFor(() => expect(save).toBeEnabled());
  await user.click(save);
  await waitFor(() => expect(payload?.defaultBranch).toBe('release'));
  // Patch omission preserves the original values; a full submission must preserve them exactly.
  if ('onClone' in payload) expect(payload.onClone).toEqual(resource.onClone);
  if ('onPull' in payload) expect(payload.onPull).toEqual(resource.onPull);
  const cloneCommands = screen.getByDisplayValue('echo clone');
  await user.clear(cloneCommands);
  await user.type(cloneCommands, 'echo updated');
  const pullPath = screen.getByDisplayValue('scripts');
  await user.clear(pullPath);
  await user.type(pullPath, 'tools');
  await waitFor(() => expect(screen.getAllByRole('button', { name: 'Save' })[0]).toBeEnabled());
  await user.click(screen.getAllByRole('button', { name: 'Save' })[0]);
  await waitFor(() => expect(payload?.onClone).toEqual({ path: 'setup', commands: ['echo updated'] }));
  expect(payload.onPull).toEqual({ path: 'tools', commands: ['echo pull'] });
});
