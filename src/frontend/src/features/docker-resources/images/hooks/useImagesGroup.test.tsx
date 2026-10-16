import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useImagesGroup } from './useImagesGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

const realtime = vi.hoisted(() => ({ options: undefined as Parameters<typeof useRealtimeGroup>[0] | undefined }));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    realtime.options = options;
  },
}));

function Probe({ platformId = 'platform-1' }: { platformId?: string }) {
  const { imagesInfo, refetch } = useImagesGroup(platformId);
  return (
    <>
      <pre data-testid="images">{JSON.stringify(imagesInfo)}</pre>
      <button onClick={() => refetch()}>Refresh</button>
    </>
  );
}
function snapshot(status = 'NotChecked', ids = ['image-1'], platformId = 'platform-1') {
  return {
    platformId,
    capabilities: { canRead: true, canPull: true },
    images: ids.map((id) => ({ id, updates: [{ reference: 'nginx:17', status }], usedBy: { containers: 2 } })),
  };
}
function subscribe() {
  const on = vi.fn();
  const off = vi.fn();
  const connection = { on, off } as unknown as RealtimeConnection;
  realtime.options!.setupEventListeners(connection);
  expect(on.mock.calls.map(([event]) => event)).toEqual(['ImagesInfoUpdated']);
  return { update: on.mock.calls[0][1], off, connection };
}

it('applies realtime bursts, usage, deletions and resync without refetching; manual refresh still reads HTTP', async () => {
  const requests = vi.fn(() => HttpResponse.json(snapshot()));
  server.use(http.get('http://localhost/api/v1/images/platform-1', requests));
  const { user, queryClient } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('NotChecked'));
  expect(realtime.options?.groupName).toBe('images:platform-1');
  const { update, off, connection } = subscribe();
  await act(async () => {
    for (let i = 0; i < 20; i++) update(snapshot(i % 2 ? 'UpToDate' : 'UpdateAvailable'));
  });
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('UpToDate'));
  expect(screen.getByTestId('images')).toHaveTextContent('"containers":2');
  expect(queryClient.getQueryData(['listImages', { platformId: 'platform-1' }])).toMatchObject({
    data: { capabilities: { canPull: true } },
  });
  await act(async () => update(snapshot('UpdateAvailable', ['foreign'], 'platform-2')));
  expect(screen.getByTestId('images')).not.toHaveTextContent('foreign');
  await act(async () => update(snapshot('UpToDate', [])));
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('"images":[]'));
  await act(async () => update(snapshot('UpToDate', ['image-2'])));
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('image-2'));
  expect(requests).toHaveBeenCalledTimes(1);
  await user.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('NotChecked'));
  expect(requests).toHaveBeenCalledTimes(2);
  realtime.options!.removeEventListeners!(connection);
  expect(off).toHaveBeenCalledWith('ImagesInfoUpdated', update);
});

it('keeps the newer realtime snapshot when the initial HTTP request finishes late', async () => {
  let release!: () => void;
  let finished!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  const completed = new Promise<void>((resolve) => {
    finished = resolve;
  });
  const requests = vi.fn(async () => {
    await pending;
    finished();
    return HttpResponse.json(snapshot());
  });
  server.use(http.get('http://localhost/api/v1/images/platform-1', requests));
  renderCitadel(<Probe />);
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(1));
  const { update } = subscribe();
  await act(async () => update(snapshot('UpToDate')));
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('UpToDate'));
  await act(async () => {
    release();
    await completed;
  });
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('UpToDate'));
  expect(requests).toHaveBeenCalledTimes(1);
});

it('isolates snapshots when switching platforms', async () => {
  server.use(
    http.get('http://localhost/api/v1/images/:platformId', ({ params }) =>
      HttpResponse.json(snapshot('NotChecked', [String(params.platformId)], String(params.platformId))),
    ),
  );
  const { rerender } = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('platform-1'));
  rerender(<Probe platformId="platform-2" />);
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('platform-2'));
  const { update } = subscribe();
  await act(async () => update(snapshot('UpToDate', ['old-platform'])));
  expect(screen.getByTestId('images')).not.toHaveTextContent('old-platform');
  await act(async () => update(snapshot('UpToDate', ['new-platform'], 'platform-2')));
  await waitFor(() => expect(screen.getByTestId('images')).toHaveTextContent('new-platform'));
});
