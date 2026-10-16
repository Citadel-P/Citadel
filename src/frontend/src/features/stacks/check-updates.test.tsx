import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { StackSource, type StackView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { StackUpdateCheckButton } from './check-updates';

vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: ({ id, selected, onSelect, disabled }: any) => (
    <select
      id={id}
      value={selected?.id ?? ''}
      disabled={disabled}
      onChange={(event) => onSelect(event.target.value ? { id: event.target.value, name: 'Docker Hub' } : undefined)}>
      <option value="">Select a Registry</option>
      <option value="registry-1">Docker Hub</option>
    </select>
  ),
}));

const stack = {
  id: 'stack-1',
  name: 'WordPress',
  rowVersion: 7,
  stackSource: 'WebEditor',
  status: 'Healthy',
  controlState: 'Idle',
  capabilities: { canWrite: true },
  spec: { $type: 'WebEditor', composeFile: 'services: {}', updateBehavior: 'Disabled' },
} as StackView;
const saved = vi.fn();
const checked = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  server.use(
    http.patch('http://localhost/api/v1/stacks/stack-1', async ({ request }) => {
      saved(await request.json());
      return HttpResponse.json({ ...stack, rowVersion: 8, spec: { ...stack.spec, registryId: 'registry-1' } });
    }),
    http.post('http://localhost/api/v1/stacks/stack-1/check-updates', () => {
      checked();
      return HttpResponse.json(stack);
    }),
  );
});

async function selectRegistry(user: ReturnType<typeof renderCitadel>['user']) {
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByRole('button', { name: 'Save Registry and check' })).toBeDisabled();
  await user.selectOptions(dialog.getByLabelText('Registry'), 'registry-1');
  return dialog;
}

it('prompts before checking and saves only the Registry with the observed version', async () => {
  const { user, queryClient } = renderCitadel(<StackUpdateCheckButton resource={stack} />);
  const invalidate = vi.spyOn(queryClient, 'invalidateQueries');
  const dialog = await selectRegistry(user);
  expect(checked).not.toHaveBeenCalled();
  expect(saved).not.toHaveBeenCalled();
  expect(dialog.getByText(/saves the Registry in the stack configuration/)).toBeVisible();
  await user.click(dialog.getByRole('button', { name: 'Save Registry and check' }));
  await waitFor(() => expect(checked).toHaveBeenCalledOnce());
  expect(saved).toHaveBeenCalledExactlyOnceWith({ rowVersion: 7, spec: { registryId: 'registry-1' } });
  expect(saved.mock.invocationCallOrder[0]).toBeLessThan(checked.mock.invocationCallOrder[0]);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ['getStackConfig'] });
  expect(invalidate.mock.calls.filter(([options]) => options?.queryKey?.[0] === 'getStackConfig')).toHaveLength(2);
});

it('cancels without saving or checking', async () => {
  const { user } = renderCitadel(<StackUpdateCheckButton resource={stack} />);
  const dialog = await selectRegistry(user);
  await user.click(dialog.getByRole('button', { name: 'Cancel' }));
  expect(saved).not.toHaveBeenCalled();
  expect(checked).not.toHaveBeenCalled();
});

it('does not check after a configuration conflict', async () => {
  server.use(
    http.patch('http://localhost/api/v1/stacks/stack-1', () =>
      HttpResponse.json({ detail: 'The Stack changed. Refresh and try again.' }, { status: 409 }),
    ),
  );
  const { user } = renderCitadel(<StackUpdateCheckButton resource={stack} />);
  const dialog = await selectRegistry(user);
  await user.click(dialog.getByRole('button', { name: 'Save Registry and check' }));
  expect(await dialog.findByRole('alert')).toHaveTextContent('The Stack changed');
  expect(checked).not.toHaveBeenCalled();
});

it('retries a failed check without saving the Registry twice', async () => {
  server.use(
    http.post(
      'http://localhost/api/v1/stacks/stack-1/check-updates',
      () => {
        checked();
        return HttpResponse.json({ detail: 'Registry unavailable.' }, { status: 502 });
      },
      { once: true },
    ),
  );
  const { user } = renderCitadel(<StackUpdateCheckButton resource={stack} />);
  const dialog = await selectRegistry(user);
  await user.click(dialog.getByRole('button', { name: 'Save Registry and check' }));
  expect(await dialog.findByRole('alert')).toHaveTextContent('Registry unavailable');
  expect(dialog.getByLabelText('Registry')).toBeDisabled();
  await user.click(dialog.getByRole('button', { name: 'Retry check' }));
  await waitFor(() => expect(checked).toHaveBeenCalledTimes(2));
  expect(saved).toHaveBeenCalledOnce();
});

const git = {
  ...stack,
  stackSource: StackSource.Git,
  spec: { $type: 'Git', gitRepoId: 'repo', branch: 'main' },
  source: { sourceType: StackSource.Git, gitRepositoryId: 'repo', branch: 'main', resolvedCommitSha: 'abc' },
} as StackView;

it.each([
  ['configured image checks', { ...stack, spec: { ...stack.spec, registryId: 'registry-1' } } as StackView],
  ['Git-only checks', git],
])('preserves direct %s', async (_label, resource) => {
  const { user } = renderCitadel(<StackUpdateCheckButton resource={resource} />);
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  await waitFor(() => expect(checked).toHaveBeenCalledOnce());
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(saved).not.toHaveBeenCalled();
});

it('prompts for a Registry for pinned Git stacks without image checks configured', async () => {
  const resource = { ...git, spec: { ...git.spec, commitSha: 'abc' } } as StackView;
  const { user } = renderCitadel(<StackUpdateCheckButton resource={resource} />);
  await selectRegistry(user);
  expect(checked).not.toHaveBeenCalled();
});

it.each([
  ['read-only', { capabilities: { canRead: true, canWrite: false } }],
  ['busy', { controlState: 'Processing' }],
  ['not deployed', { status: 'Created' }],
])('blocks %s stacks', (_label, changes) => {
  renderCitadel(<StackUpdateCheckButton resource={{ ...stack, ...changes } as StackView} />);
  expect(screen.getByRole('button', { name: 'Check for updates' })).toBeDisabled();
  expect(saved).not.toHaveBeenCalled();
  expect(checked).not.toHaveBeenCalled();
});
