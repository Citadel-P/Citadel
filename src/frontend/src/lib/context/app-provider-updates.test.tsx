import { act, screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import type { ReactNode } from 'react';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AppProvider } from './app-provider';
import { useAppContext } from './app-context';

vi.mock('@/features/platforms/hooks/usePlatformsGroup', () => ({
  usePlatformsGroup: () => ({ platformsMessage: undefined, isLoading: false }),
}));
vi.mock('@/features/alerters/alert-events/hooks/useAlertEventsGroup', () => ({
  useAlertEventsGroup: () => ({}),
}));
vi.mock('./realtime-provider', () => ({ RealtimeProvider: ({ children }: { children: ReactNode }) => children }));

function UpdateNotice() {
  const { applicationInfo } = useAppContext();
  return <span>{applicationInfo ? (applicationInfo.availableUpdate?.version ?? 'No update') : 'Loading'}</span>;
}

it('picks up a background release check after the initial application-info read', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  let completed = false;
  const read = vi.fn(() =>
    HttpResponse.json({
      name: 'Citadel',
      version: '0.1.0',
      informationalVersion: '0.1.0',
      realtimeTransport: 'WebSocketV1',
      ...(completed ? { availableUpdate: { version: '0.1.1', releaseUrl: 'https://example.test/release' } } : {}),
    }),
  );
  server.use(http.get('http://localhost/api/v1/application/info', read));
  const { unmount } = renderCitadel(
    <AppProvider>
      <UpdateNotice />
    </AppProvider>,
  );
  try {
    expect(await screen.findByText('No update')).toBeVisible();
    expect(read).toHaveBeenCalledOnce();
    completed = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(await screen.findByText('0.1.1')).toBeVisible();
    expect(read).toHaveBeenCalledTimes(2);
  } finally {
    unmount();
    vi.useRealTimers();
  }
});
