import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ImageInfoComponents } from './index';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { useRealtimeGroup } from '@/hooks/useRealtimeGroup';

const realtime = vi.hoisted(() => ({ options: undefined as Parameters<typeof useRealtimeGroup>[0] | undefined }));
vi.mock('@/hooks/useRealtimeGroup', () => ({
  useRealtimeGroup: (options: Parameters<typeof useRealtimeGroup>[0]) => {
    realtime.options = options;
  },
}));

it('reloads image inspection when a scheduled check or pull invalidates images', async () => {
  let status = 'UpdateAvailable';
  const requests = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/images/platform-1/image-1', () => {
      requests();
      return HttpResponse.json({ id: 'image-1', updates: [{ reference: 'nginx:17', status }] });
    }),
  );
  function Inspection() {
    const useImage = ImageInfoComponents.useData;
    const { resource } = useImage('platform-1', 'image-1');
    return <span>{resource?.updates?.[0]?.status ?? 'Loading'}</span>;
  }
  renderCitadel(<Inspection />);
  expect(await screen.findByText('UpdateAvailable')).toBeVisible();
  expect(realtime.options?.groupName).toBe('images:platform-1');
  const on = vi.fn();
  const off = vi.fn();
  const connection = { on, off } as unknown as RealtimeConnection;
  realtime.options!.setupEventListeners(connection);
  const [event, refresh] = on.mock.calls[0];
  expect(event).toBe('ImagesInfoUpdated');
  status = 'UpToDate';
  await act(async () => refresh());
  expect(await screen.findByText('UpToDate')).toBeVisible();
  await waitFor(() => expect(requests).toHaveBeenCalledTimes(2));
  realtime.options!.removeEventListeners!(connection);
  expect(off).toHaveBeenCalledWith('ImagesInfoUpdated', refresh);
});
