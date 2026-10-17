import { ResourceControlState, type ImageInspectionView, type ImageView } from '@/api/generated/api.types';
import { renderCitadel as renderWithProviders } from '@/test/render-citadel';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { server } from '@/test/server';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { ImageGroupActions } from './actions';
import { ImageInfoActions } from './image-info/actions';
import { ImageInfoComponents } from './image-info';

function renderCitadel(...[ui, options]: Parameters<typeof renderWithProviders>) {
  const fake = new FakeRealtimeConnection();
  return renderWithProviders(ui, {
    groups: {
      connectionFactory: () => fake.asRealtimeConnection(),
      startConnection: (connection) => connection.start(),
    },
    ...options,
  });
}

vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: { id: 'platform-1' } }) }));

const image = {
  id: 'image-1',
  name: 'app:latest',
  dockerImageId: `sha256:${'a'.repeat(64)}`,
  dockerNodeId: null,
  capabilities: null,
  controlState: ResourceControlState.Idle,
  isInUse: false,
  isStale: false,
} as ImageView;

const deletableImage: ImageView = {
  ...image,
  capabilities: { canRead: true, canWrite: true, canExecute: true, canPull: true, canInspect: true },
};

const inspectedImage: ImageInspectionView = {
  updates: [],
  id: image.dockerImageId,
  name: image.name,
  tag: 'latest',
  capabilities: deletableImage.capabilities,
  architecture: 'amd64',
  os: 'linux',
  cmd: [],
  containers: [],
  created: '2026-10-15T12:00:00Z',
  entryPoint: [],
  env: [],
  exposedPorts: [],
  labels: {},
  layers: [],
  repoTags: ['app:latest'],
  size: 1024,
  volumes: [],
};

const CurrentLocation = () => {
  const location = useLocation();
  return <output data-testid="location">{location.pathname}</output>;
};

const previewRequests = vi.fn();
beforeEach(() => {
  previewRequests.mockClear();
  server.use(
    http.post('http://localhost/api/v1/images/deletion-preview', () => {
      previewRequests();
      return HttpResponse.json({}, { status: 500 });
    }),
  );
});
afterEach(() => expect(previewRequests).not.toHaveBeenCalled());

describe('Image capabilities', () => {
  it.each([null, false, true])('requires an explicit inspect capability (%s)', (allowed) => {
    const Inspect = ImageGroupActions.inspect;
    renderCitadel(
      <Inspect
        resources={[
          {
            ...image,
            capabilities:
              allowed === null
                ? null
                : {
                    canRead: true,
                    canWrite: false,
                    canExecute: false,
                    canPull: false,
                    canInspect: allowed,
                  },
          },
        ]}
      />,
    );
    const button = screen.getByRole('button', { name: /inspect/i });
    if (allowed === true) expect(button).toBeEnabled();
    else expect(button).toBeDisabled();
  });
});

describe.each(['list', 'detail'] as const)('Image deletion from %s', (surface) => {
  const Delete = ({ resource }: { resource: ImageView }) =>
    surface === 'list' ? (
      <ImageGroupActions.delete resources={[resource]} />
    ) : (
      <ImageInfoActions.delete resource={resource} />
    );

  it.each([
    ['missing permissions', { capabilities: null }],
    ['denied execution', { capabilities: { ...deletableImage.capabilities!, canExecute: false } }],
    ['node-local image', { dockerNodeId: 'worker-1' }],
    ['stale inventory', { isStale: true }],
    ['container reference', { isInUse: true }],
    ['operation in progress', { controlState: ResourceControlState.Processing }],
  ] satisfies [string, Partial<ImageView>][])('blocks deletion for %s', (_reason, changes) => {
    renderCitadel(<Delete resource={{ ...deletableImage, ...changes }} />);
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('uses non-forced deletion without pruning parents', async () => {
    const deleted = vi.fn();
    server.use(
      http.delete('http://localhost/api/v1/images', async ({ request }) => {
        deleted(await request.json());
        return HttpResponse.json({ deletedImages: [] });
      }),
    );
    const { user } = renderCitadel(
      <>
        <Delete resource={deletableImage} />
        <CurrentLocation />
      </>,
      {
        route: '/platforms/platform-1/images/image-1',
      },
    );

    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = within(screen.getByRole('dialog'));
    await user.type(dialog.getByRole('textbox'), surface === 'list' ? 'Delete' : image.name);
    await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
    await user.click(dialog.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(deleted).toHaveBeenCalledExactlyOnceWith({
        platformId: 'platform-1',
        ids: [image.dockerImageId],
        force: false,
        noPrune: true,
      }),
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    if (surface === 'detail') expect(screen.getByTestId('location').textContent).toBe('/platforms/platform-1/images');
  });

  it('keeps a Docker conflict visible without retrying with force', async () => {
    const deleted = vi.fn();
    server.use(
      http.delete('http://localhost/api/v1/images', async ({ request }) => {
        deleted(await request.json());
        return HttpResponse.json({ title: 'Image has multiple tags', status: 409 }, { status: 409 });
      }),
    );
    const route = '/platforms/platform-1/images/image-1';
    const { user } = renderCitadel(
      <>
        <Delete resource={deletableImage} />
        <CurrentLocation />
      </>,
      { route },
    );
    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = within(screen.getByRole('dialog'));
    await user.type(dialog.getByRole('textbox'), surface === 'list' ? 'Delete' : image.name);
    await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
    await user.click(dialog.getByRole('button', { name: 'Delete' }));

    await waitFor(() => expect(deleted).toHaveBeenCalledTimes(1));
    expect(await dialog.findByText('Image has multiple tags')).toBeVisible();
    expect(deleted.mock.calls[0][0]).toMatchObject({ force: false, noPrune: true });
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(screen.getByTestId('location').textContent).toBe(route);

    expect(deleted).toHaveBeenCalledTimes(1);
  });
});

describe('Image deletion selection and inspection', () => {
  it('blocks an empty selection', () => {
    renderCitadel(<ImageGroupActions.delete resources={[]} />);
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('blocks the whole selection when one image cannot be deleted', () => {
    renderCitadel(
      <ImageGroupActions.delete resources={[deletableImage, { ...deletableImage, dockerNodeId: 'worker-1' }]} />,
    );
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('deduplicates image IDs before dispatch', async () => {
    const deleted = vi.fn();
    server.use(
      http.delete('http://localhost/api/v1/images', async ({ request }) => {
        deleted(await request.json());
        return HttpResponse.json({ deletedImages: [] });
      }),
    );
    const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage, deletableImage]} />);
    await user.click(screen.getByRole('button', { name: 'Delete' }));
    const dialog = within(screen.getByRole('dialog'));
    await user.type(dialog.getByRole('textbox'), 'Delete');
    await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
    await user.click(dialog.getByRole('button', { name: 'Delete' }));
    await waitFor(() =>
      expect(deleted).toHaveBeenCalledExactlyOnceWith({
        platformId: 'platform-1',
        ids: [image.dockerImageId],
        force: false,
        noPrune: true,
      }),
    );
  });

  it.each(['running', 'exited'])('passes %s container usage from image inspection to the delete action', (state) => {
    const DetailsActions = ImageInfoComponents.Header.ActionButtons;
    const inspection: ImageInspectionView = {
      ...inspectedImage,
      containers: [{ id: 'container-1', name: 'app', state, volumes: [], networks: {}, ports: {} }],
    };
    renderCitadel(<DetailsActions resource={inspection} />);
    expect(screen.getByRole('button', { name: 'Delete' })).toBeDisabled();
  });

  it('allows deletion after successful inspection with no known containers', () => {
    const DetailsActions = ImageInfoComponents.Header.ActionButtons;
    renderCitadel(<DetailsActions resource={inspectedImage} />);
    expect(screen.getByRole('button', { name: 'Delete' })).toBeEnabled();
  });
});

it('shows a dependency conflict from deletion and keeps the dialog open', async () => {
  const deleted = vi.fn();
  server.use(
    http.delete('http://localhost/api/v1/images', () => {
      deleted();
      return HttpResponse.json(
        { title: 'A saved Local image dependency prevents deletion.', status: 409 },
        { status: 409 },
      );
    }),
  );
  const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByRole('textbox'), 'Delete');
  await user.click(dialog.getByRole('button', { name: 'Delete' }));
  expect(await dialog.findByText('A saved Local image dependency prevents deletion.')).toBeVisible();
  expect(screen.getByRole('dialog')).toBeVisible();
  expect(deleted).toHaveBeenCalledTimes(1);
});

it('blocks deletion when the selection changes while the dialog is open', async () => {
  const { user, rerender } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByRole('textbox'), 'Delete');
  await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
  rerender(<ImageGroupActions.delete resources={[{ ...deletableImage, dockerImageId: `sha256:${'b'.repeat(64)}` }]} />);
  expect(screen.getByText(/The selection or Platform changed/)).toBeVisible();
  expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
});

it('keeps the confirmation compact without fetching usage pages', async () => {
  const usageRequests = vi.fn();
  server.use(
    http.get(`http://localhost/api/v1/images/platform-1/${image.dockerImageId}/usage`, () => {
      usageRequests();
      return HttpResponse.json({ entries: [], nextOffset: null, coverageComplete: false, limitations: [] });
    }),
  );
  const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByRole('textbox'), 'Delete');
  await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
  expect(dialog.getByText(image.name)).toBeVisible();
  expect(dialog.queryByText(image.dockerImageId)).not.toBeInTheDocument();
  expect(
    dialog.queryByText(/Reviewed|registry artifact|No known references|Some image references/),
  ).not.toBeInTheDocument();
  expect(dialog.queryByRole('button', { name: /More references|Check again/ })).not.toBeInTheDocument();
  expect(usageRequests).not.toHaveBeenCalled();
});

it('uses the shared multi-delete list and click-to-copy confirmation', async () => {
  const otherImage = { ...deletableImage, name: 'redis:latest', dockerImageId: `sha256:${'b'.repeat(64)}` };
  const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage, otherImage]} />);
  const copy = vi.spyOn(navigator.clipboard, 'writeText');
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByRole('heading', { name: 'Confirm Delete' })).toBeVisible();
  expect(dialog.getAllByRole('listitem').map((item) => item.textContent)).toEqual(['app:latest', 'redis:latest']);
  expect(dialog.getByText('You may click the name in bold to copy it')).toBeVisible();
  await user.click(dialog.getByText('Delete', { selector: 'b' }));
  expect(copy).toHaveBeenCalledWith('Delete');
  expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
  await user.type(dialog.getByRole('textbox', { name: 'Enter Delete to confirm' }), 'Delete');
  await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
});
