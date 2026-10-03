import { renderCitadel } from '@/test/render-citadel';
import type { ReactNode } from 'react';
import { Outlet, Route, Routes } from 'react-router';
import { beforeEach, vi } from 'vitest';
import { useAppContext } from './app-context';
import { AppProvider } from './app-provider';

const mocks = vi.hoisted(() => ({
  useRead: vi.fn(),
  usePlatformsGroup: vi.fn(),
  useAlertEventsGroup: vi.fn(),
}));

vi.mock('../hooks', () => ({ useRead: mocks.useRead }));
vi.mock('@/features/platforms/hooks/usePlatformsGroup', () => ({
  usePlatformsGroup: mocks.usePlatformsGroup,
}));
vi.mock('@/features/alerters/alert-events/hooks/useAlertEventsGroup', () => ({
  useAlertEventsGroup: mocks.useAlertEventsGroup,
}));
vi.mock('./realtime-provider', () => ({ RealtimeProvider: ({ children }: { children: ReactNode }) => children }));

describe('AppProvider', () => {
  beforeEach(() => {
    mocks.useRead.mockReset();
    mocks.useRead.mockReturnValue({ data: undefined, isLoading: false });
    mocks.usePlatformsGroup.mockReturnValue({ platformsMessage: undefined, isLoading: false });
    mocks.useAlertEventsGroup.mockReturnValue({});
  });

  it('loads the selected platform from its edit route', () => {
    renderCitadel(
      <Routes>
        <Route
          path="/"
          element={
            <AppProvider>
              <Outlet />
            </AppProvider>
          }>
          <Route path=":type/edit/:id" element={<CurrentPlatform />} />
        </Route>
      </Routes>,
      { route: '/platforms/edit/swarm-1' },
    );

    expect(mocks.useRead).toHaveBeenCalledWith('getPlatfom', { id: 'swarm-1' });
  });
});

const CurrentPlatform = () => {
  const { currentPlatform } = useAppContext();
  return <div>{currentPlatform?.name}</div>;
};
