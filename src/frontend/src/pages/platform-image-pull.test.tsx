import { act, screen, waitFor } from '@testing-library/react';
import { Provider } from 'jotai';
import { http, HttpResponse } from 'msw';
import { Route, Routes, useNavigate } from 'react-router';
import { PlatformType } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import PlatformResourcePage from './platform-resource';

vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({
    currentPlatform: {
      id: 'platform-1',
      name: 'Host',
      type: PlatformType.Docker,
      capabilities: { canRead: true, canPull: true },
    },
    isLoading: false,
  }),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => {} }));
vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));

it('starts pulling from image details immediately and keeps one stream when navigating to the list and back', async () => {
  const imageId = `sha256:${'a'.repeat(64)}`;
  const capabilities = { canRead: true, canPull: true, canInspect: true, canWrite: false, canExecute: false };
  const image = {
    id: imageId,
    name: 'nginx',
    tag: '17',
    repoTags: ['nginx:17'],
    size: 1024,
    os: 'linux',
    architecture: 'amd64',
    created: '2026-10-16T09:00:00Z',
    containers: [],
    layers: [],
    labels: {},
    exposedPorts: [],
    cmd: [],
    entryPoint: [],
    env: [],
    volumes: [],
    capabilities,
  };
  const requests: unknown[] = [];
  const listed = vi.fn();
  let aborted = false;
  let finish!: () => void;
  const completed = new Promise<void>((resolve) => {
    finish = resolve;
  });
  server.use(
    http.get('*/api/v1/images/platform-1/:imageId', () => HttpResponse.json(image)),
    http.get('*/api/v1/images/platform-1/:imageId/usage', () =>
      HttpResponse.json({ entries: [], limitations: [], coverageComplete: true }),
    ),
    http.get('*/api/v1/images/platform-1', () => {
      listed();
      return HttpResponse.json({ images: [], capabilities });
    }),
    http.post('*/api/v1/images/pull', async ({ request }) => {
      requests.push(await request.json());
      request.signal.addEventListener('abort', () => {
        aborted = true;
      });
      await completed;
      return HttpResponse.json([{ dockerImageId: 'pulled-image' }]);
    }),
  );
  let navigate!: ReturnType<typeof useNavigate>;
  function Page() {
    navigate = useNavigate();
    return <PlatformResourcePage />;
  }
  const { user } = renderCitadel(
    <Provider>
      <Routes>
        <Route path="/platforms/:platformId/:type/:resourceId?" element={<Page />} />
      </Routes>
    </Provider>,
    { route: '/platforms/platform-1/images/aaaaaaaaaaaa' },
  );
  try {
    await user.click(await screen.findByRole('button', { name: 'Pull latest' }));
    const sheet = await screen.findByRole('dialog', { name: 'Pull Image' });
    await waitFor(() =>
      expect(requests).toEqual([{ platformId: 'platform-1', sourceImageId: imageId, imageTag: 'nginx:17' }]),
    );
    await act(async () => {
      await navigate('/platforms/platform-1/images');
    });
    await waitFor(() => expect(listed).toHaveBeenCalled());
    expect(screen.getByRole('dialog', { name: 'Pull Image' })).toBe(sheet);
    expect(requests).toHaveLength(1);
    expect(aborted).toBe(false);
    finish();
    await act(async () => {
      await navigate('/platforms/platform-1/images/aaaaaaaaaaaa');
    });
    await waitFor(() =>
      expect(screen.getByRole('heading', { name: 'Image details', hidden: true })).toBeInTheDocument(),
    );
    expect(screen.getByRole('dialog', { name: 'Pull Image' })).toBe(sheet);
    expect(requests).toHaveLength(1);
    expect(screen.queryByRole('link', { name: 'Review workload usage' })).not.toBeInTheDocument();
  } finally {
    finish();
  }
});
