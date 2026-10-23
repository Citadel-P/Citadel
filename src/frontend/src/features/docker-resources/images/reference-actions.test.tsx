import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent } from '@/components/ui/dropdown-menu';
import { screen, within, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ImageReferenceButton, ImageReferenceDropdown, type ImageReferenceTarget } from './reference-actions';
const { openTask, platform } = vi.hoisted(() => ({
  openTask: vi.fn(),
  platform: { id: 'platform-1', name: 'Production host', capabilities: { canRead: true, canPull: true } },
}));
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({
    currentPlatform: platform,
  }),
}));
vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({ open: openTask }),
  useInlineSubHeader: () => ({ open: false }),
}));
vi.mock('@/components/custom/common', async (original) => {
  const actual = await original<typeof import('@/components/custom/common')>();
  return {
    ...actual,
    ResourceSelectorField: (props: Parameters<typeof actual.ResourceSelectorField>[0]) =>
      props.items ? (
        <actual.ResourceSelectorField {...props} />
      ) : (
        <>
          <button
            id={props.id}
            role="combobox"
            aria-expanded="false"
            aria-controls={`${props.id}-options`}
            disabled={props.disabled}
            onClick={() => props.onSelect?.({ id: 'registry-1', name: 'Docker Hub' })}>
            Choose Registry
          </button>
          <div id={`${props.id}-options`} hidden />
        </>
      ),
  };
});
const image: ImageReferenceTarget = {
  dockerImageId: `sha256:${'a'.repeat(64)}`,
  tags: ['nginx:17'],
  isStale: false,
  capabilities: { canRead: true, canWrite: false, canExecute: false, canInspect: true, canPull: true },
};
beforeEach(() => {
  openTask.mockClear();
  platform.id = 'platform-1';
  platform.capabilities.canPull = true;
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));
});
it('checks the selected tag without dispatching a pull or a workload command', async () => {
  const checked = vi.fn();
  server.use(
    http.post('http://localhost/api/v1/images/check-update', async ({ request }) => {
      checked(await request.json());
      return HttpResponse.json({
        reference: 'nginx:17',
        registryId: 'registry-1',
        status: 'UpdateAvailable',
        reason: 'The registry currently publishes different content for this tag.',
        localConfigDigest: 'sha256:local',
        remoteConfigDigest: 'sha256:remote',
        isStale: false,
      });
    }),
  );
  const { user } = renderCitadel(<ImageReferenceButton resources={[image]} kind="check" />);
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByLabelText('Image tag')).toHaveTextContent('nginx:17');
  expect(dialog.getByRole('heading', { name: 'Registry credentials' })).toBeVisible();
  expect(dialog.getByRole('link', { name: 'Manage Registries' })).toHaveAttribute('href', '/registries');
  expect(dialog.getByRole('button', { name: 'Check for updates' })).toBeDisabled();
  await user.click(dialog.getByRole('combobox', { name: 'Registry' }));
  await user.click(dialog.getByRole('button', { name: 'Check for updates' }));
  expect(await dialog.findByText('Update available')).toBeVisible();
  expect(dialog.getByRole('heading', { name: 'Check result' })).toBeVisible();
  expect(dialog.getByText(/The registry has a different image for this tag/)).toBeVisible();
  expect(dialog.queryByText('sha256:remote')).not.toBeInTheDocument();
  expect(dialog.getByRole('button', { name: 'Check again' })).toBeEnabled();
  await user.click(dialog.getByRole('button', { name: 'Technical details' }));
  expect(dialog.getByText('sha256:remote')).toBeVisible();
  expect(checked).toHaveBeenCalledExactlyOnceWith({
    platformId: 'platform-1',
    dockerImageId: image.dockerImageId,
    registryId: 'registry-1',
    reference: 'nginx:17',
  });
  expect(openTask).not.toHaveBeenCalled();
});
it('offers an explicit pull of the checked tag and Registry after finding an update', async () => {
  server.use(
    http.post('http://localhost/api/v1/images/check-update', () =>
      HttpResponse.json({ reference: 'nginx:17', registryId: 'registry-1', status: 'UpdateAvailable', isStale: false }),
    ),
  );
  const { user } = renderCitadel(
    <ImageReferenceButton resources={[{ ...image, tags: ['nginx:17', 'nginx:latest'] }]} kind="check" />,
  );
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.queryByRole('button', { name: 'Pull latest' })).not.toBeInTheDocument();
  await user.click(dialog.getByRole('combobox', { name: 'Image tag' }));
  await user.click(screen.getByRole('option', { name: 'nginx:17' }));
  await user.click(dialog.getByRole('combobox', { name: 'Registry' }));
  await user.click(dialog.getByRole('button', { name: 'Check for updates' }));
  const pull = await dialog.findByRole('button', { name: 'Pull latest' });
  expect(openTask).not.toHaveBeenCalled();
  await user.click(pull);
  expect(openTask).toHaveBeenCalledExactlyOnceWith({
    kind: 'pull',
    payload: {
      platformId: 'platform-1',
      sourceImageId: image.dockerImageId,
      imageTag: 'nginx:17',
      registryId: 'registry-1',
    },
  });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it.each(['image permission', 'platform permission', 'stale result', 'up to date', 'failed check'])(
  'does not allow pulling the update with %s',
  async (condition) => {
    if (condition === 'platform permission') platform.capabilities.canPull = false;
    server.use(
      http.post('http://localhost/api/v1/images/check-update', () =>
        HttpResponse.json({
          reference: 'nginx:17',
          status:
            condition === 'up to date' ? 'UpToDate' : condition === 'failed check' ? 'CheckFailed' : 'UpdateAvailable',
          isStale: condition === 'stale result',
        }),
      ),
    );
    const { user } = renderCitadel(
      <ImageReferenceButton
        resources={[
          condition === 'image permission'
            ? { ...image, capabilities: { ...image.capabilities!, canPull: false } }
            : image,
        ]}
        kind="check"
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Check for updates' }));
    const dialog = within(screen.getByRole('dialog'));
    await user.click(dialog.getByRole('combobox', { name: 'Registry' }));
    await user.click(dialog.getByRole('button', { name: 'Check for updates' }));
    await dialog.findByRole('heading', { name: 'Check result' });
    const pull = dialog.queryByRole('button', { name: 'Pull latest' });
    if (condition.endsWith('permission')) expect(pull).toBeDisabled();
    else expect(pull).not.toBeInTheDocument();
    expect(openTask).not.toHaveBeenCalled();
  },
);

it('clears the pull action when the checked tag selection changes', async () => {
  server.use(
    http.post('http://localhost/api/v1/images/check-update', () =>
      HttpResponse.json({ reference: 'nginx:17', status: 'UpdateAvailable', isStale: false }),
    ),
  );
  const { user } = renderCitadel(
    <ImageReferenceButton resources={[{ ...image, tags: ['nginx:17', 'nginx:latest'] }]} kind="check" />,
  );
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.click(dialog.getByRole('combobox', { name: 'Image tag' }));
  await user.click(screen.getByRole('option', { name: 'nginx:17' }));
  await user.click(dialog.getByRole('combobox', { name: 'Registry' }));
  await user.click(dialog.getByRole('button', { name: 'Check for updates' }));
  expect(await dialog.findByRole('button', { name: 'Pull latest' })).toBeEnabled();
  await user.click(dialog.getByRole('combobox', { name: 'Image tag' }));
  await user.click(screen.getByRole('option', { name: 'nginx:latest' }));
  expect(dialog.queryByRole('button', { name: 'Pull latest' })).not.toBeInTheDocument();
  expect(openTask).not.toHaveBeenCalled();
});

it('requires a tag choice and prefills the existing pull task without replacing it with latest', async () => {
  const { user } = renderCitadel(
    <ImageReferenceButton resources={[{ ...image, tags: ['nginx:17', 'nginx:latest'] }]} kind="pull" />,
  );
  await user.click(screen.getByRole('button', { name: 'Pull latest' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByRole('combobox', { name: 'Image tag' })).toHaveTextContent('Select a tag');
  expect(dialog.getByRole('button', { name: 'Pull latest' })).toBeDisabled();
  expect(dialog.queryByRole('combobox', { name: 'Registry' })).not.toBeInTheDocument();
  await user.click(dialog.getByRole('combobox', { name: 'Image tag' }));
  await user.click(screen.getByRole('option', { name: 'nginx:17' }));
  await user.click(dialog.getByRole('button', { name: 'Pull latest' }));
  await waitFor(() =>
    expect(openTask).toHaveBeenCalledExactlyOnceWith({
      kind: 'pull',
      payload: {
        platformId: 'platform-1',
        sourceImageId: image.dockerImageId,
        imageTag: 'nginx:17',
      },
    }),
  );
});
it.each([
  ['node-local', { dockerNodeId: 'worker-1' }],
  ['stale', { isStale: true }],
  ['missing permission', { capabilities: null }],
  ['untagged', { tags: [] }],
])('blocks contextual pull for %s images', (_name, changes) => {
  renderCitadel(<ImageReferenceButton resources={[{ ...image, ...changes }]} kind="pull" />);
  expect(screen.getByRole('button', { name: 'Pull latest' })).toBeDisabled();
});

it('displays a denied registry check without reporting an update', async () => {
  const reason =
    'Access to the host or Registry was denied. Verify Registry credentials and that the repository/tag is published and readable.';
  server.use(
    http.post('http://localhost/api/v1/images/check-update', () =>
      HttpResponse.json({
        reference: 'citadel-agent:local',
        status: 'CheckFailed',
        reason,
        isStale: false,
      }),
    ),
  );
  const { user } = renderCitadel(
    <ImageReferenceButton resources={[{ ...image, tags: ['citadel-agent:local'] }]} kind="check" />,
  );
  await user.click(screen.getByRole('button', { name: 'Check for updates' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByText(/This check does not download or deploy anything/)).toBeVisible();
  await user.click(dialog.getByRole('combobox', { name: 'Registry' }));
  await user.click(dialog.getByRole('button', { name: 'Check for updates' }));
  expect(await dialog.findByText('Check failed')).toBeVisible();
  expect(dialog.getByText(reason)).toBeVisible();
  expect(dialog.queryByText('Up to date')).not.toBeInTheDocument();
  expect(openTask).not.toHaveBeenCalled();
});

it.each(['button', 'menu'])(
  'opens progress directly from the %s without requiring a configured Registry',
  async (surface) => {
    const onAction = vi.fn();
    const { user } = renderCitadel(
      surface === 'button' ? (
        <ImageReferenceButton resources={[image]} kind="pull" />
      ) : (
        <DropdownMenu>
          <DropdownMenuTrigger>Actions</DropdownMenuTrigger>
          <DropdownMenuContent>
            <ImageReferenceDropdown resource={image} kind="pull" onAction={onAction} />
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    );
    if (surface === 'menu') await user.click(screen.getByRole('button', { name: 'Actions' }));
    await user.click(screen.getByText('Pull latest'));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(onAction).not.toHaveBeenCalled();
    expect(openTask).toHaveBeenCalledExactlyOnceWith({
      kind: 'pull',
      payload: {
        platformId: 'platform-1',
        sourceImageId: image.dockerImageId,
        imageTag: 'nginx:17',
      },
    });
  },
);
