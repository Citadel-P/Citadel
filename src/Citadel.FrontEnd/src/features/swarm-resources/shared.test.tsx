import { PlatformStatus, PlatformType, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { SwarmLogs } from './shared';

const { useReadMock } = vi.hoisted(() => ({
  useReadMock: vi.fn(() => ({
    data: { data: { lines: ['service log'], truncated: false } },
    error: undefined,
  })),
}));

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: useReadMock,
}));

vi.mock('@/components/custom/common', () => ({
  LogViewer: ({ logs }: { logs: string[] }) => <div>{logs.join('\n')}</div>,
}));

const platformId = '019f0000-0000-7000-8000-000000000010';

const appContext = (status: PlatformStatus) => ({
  isLoading: false,
  currentPlatform: undefined,
  platforms: [
    {
      id: platformId,
      type: PlatformType.DockerSwarm,
      status,
    } as PlatformView,
  ],
  unresolvedAlertCount: 0,
  liveAlertEvents: {},
  receivedAlertEventIds: [],
});

describe('SwarmLogs', () => {
  beforeEach(() => useReadMock.mockClear());

  it('loads logs for an online Platform from a global resource route', () => {
    renderCitadel(
      <AppContext.Provider value={appContext(PlatformStatus.Online)}>
        <SwarmLogs
          platformId={platformId}
          resourceId="service-1"
          resource="service"
          capabilities={{ canViewLogs: true }}
        />
      </AppContext.Provider>,
    );

    expect(screen.getByText('service log')).toBeVisible();
    expect(useReadMock).toHaveBeenCalledWith(
      'getSwarmServiceLogs',
      { platformId, resourceId: 'service-1', query: { tail: 100 } },
      { enabled: true },
    );
  });

  it('does not request logs when the matching Platform is offline', () => {
    renderCitadel(
      <AppContext.Provider value={appContext(PlatformStatus.Offline)}>
        <SwarmLogs
          platformId={platformId}
          resourceId="service-1"
          resource="service"
          capabilities={{ canViewLogs: true }}
        />
      </AppContext.Provider>,
    );

    expect(screen.queryByText('service log')).not.toBeInTheDocument();
    expect(useReadMock).toHaveBeenCalledWith(
      'getSwarmServiceLogs',
      { platformId, resourceId: 'service-1', query: { tail: 100 } },
      { enabled: false },
    );
  });
});
