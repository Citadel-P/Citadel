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

const review = { dockerImageId: image.dockerImageId, rowVersion: 1, tags: ['app:latest'] };
beforeEach(() => {
  server.use(
    http.post('http://localhost/api/v1/images/deletion-preview', () =>
      HttpResponse.json([
        {
          review,
          canDelete: true,
          blockers: [],
          warnings: [],
          usage: { entries: [], nextOffset: null, coverageComplete: false, limitations: [] },
          observedAt: '2026-10-15T12:00:00Z',
        },
      ]),
    ),
  );
});

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
        reviews: [review],
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
    await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled());
    expect(dialog.getByText(/review could not be completed/)).toBeVisible();
    expect(deleted.mock.calls[0][0]).toMatchObject({ force: false, noPrune: true });
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(screen.getByTestId('location').textContent).toBe(route);
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
        reviews: [review],
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

it('shows the backend dependency blocker and never submits deletion', async () => {
  const deleted = vi.fn();
  server.use(
    http.post('http://localhost/api/v1/images/deletion-preview', () =>
      HttpResponse.json([
        {
          review,
          canDelete: false,
          blockers: ['A saved Local image dependency prevents deletion.'],
          warnings: [],
          usage: { entries: [], limitations: [], coverageComplete: false, nextOffset: null },
          observedAt: '2026-10-15T00:00:00Z',
        },
      ]),
    ),
    http.delete('http://localhost/api/v1/images', () => {
      deleted();
      return HttpResponse.json({ items: [] });
    }),
  );
  const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  expect(await screen.findByText('A saved Local image dependency prevents deletion.')).toBeVisible();
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByRole('textbox'), 'Delete');
  expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
  expect(deleted).not.toHaveBeenCalled();
});

it('requires a new review when the selection changes while the dialog is open', async () => {
  const { user, rerender } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.type(dialog.getByRole('textbox'), 'Delete');
  await waitFor(() => expect(dialog.getByRole('button', { name: 'Delete' })).toBeEnabled());
  rerender(<ImageGroupActions.delete resources={[{ ...deletableImage, dockerImageId: `sha256:${'b'.repeat(64)}` }]} />);
  expect(screen.getByText(/The selection or Platform changed/)).toBeVisible();
  expect(dialog.getByRole('button', { name: 'Delete' })).toBeDisabled();
});

it('shows additional usage pages inside deletion review', async () => {
  server.use(
    http.post('http://localhost/api/v1/images/deletion-preview', () =>
      HttpResponse.json([
        {
          review,
          canDelete: true,
          blockers: [],
          warnings: [],
          usage: { entries: [], nextOffset: 500, coverageComplete: false, limitations: [] },
          observedAt: '2026-10-15T12:00:00Z',
        },
      ]),
    ),
    http.get(`http://localhost/api/v1/images/platform-1/${image.dockerImageId}/usage`, ({ request }) => {
      expect(new URL(request.url).searchParams.get('offset')).toBe('500');
      return HttpResponse.json({
        entries: [
          {
            relationship: 'SavedTag',
            workloadKind: 'Stack',
            workloadId: 'stack-1',
            workloadName: 'Referenced stack',
            reference: 'app:latest',
            isStale: false,
          },
        ],
        nextOffset: null,
        coverageComplete: false,
        limitations: [],
      });
    }),
  );
  const { user } = renderCitadel(<ImageGroupActions.delete resources={[deletableImage]} />);
  await user.click(screen.getByRole('button', { name: 'Delete' }));
  const dialog = within(screen.getByRole('dialog'));
  await user.click(await dialog.findByRole('button', { name: 'More references' }));
  expect(await dialog.findByRole('link', { name: 'Referenced stack' })).toBeVisible();
});
