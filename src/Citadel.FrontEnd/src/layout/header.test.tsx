import { AppearanceProvider } from '@/lib/appearance/appearance-provider';
import { screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useLocation } from 'react-router';
import { Provider } from 'jotai';
import {
  AlertEventStatus,
  AlertResourceType,
  AlertSeverity,
  AlertType,
  type AlertEventView,
} from '@/api/generated/api.types';
import { AlertTaskSheet } from '@/features/alerters/alert-events/alert-task-sheet';
import { AppContext } from '@/lib/context/app-context';
import { LayoutContext } from '@/lib/context/layout-context';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { AlertBell, Header } from './header';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoDiff: () => null,
}));

vi.mock('@/components/ui/sidebar', () => ({
  SidebarTrigger: () => <button aria-label="Toggle sidebar" />,
}));

vi.mock('@/features/search/global-search', () => ({
  GlobalSearch: () => <span data-testid="global-search" />,
}));

vi.mock('./breadcrumb', () => ({
  BreadcrumbTrail: () => <span data-testid="breadcrumb" />,
}));

vi.mock('./live-connection-indicator', () => ({
  LiveConnectionIndicator: () => <span data-testid="live-connection-indicator" />,
}));

const alertEvent: AlertEventView = {
  id: '00000000-0000-0000-0000-000000000101',
  alertRuleId: '10000000-0000-0000-0000-000000000001',
  type: AlertType.PlatformUnreachable,
  severity: AlertSeverity.Warning,
  status: AlertEventStatus.Active,
  message: 'The platform is unreachable.',
  info: { $type: AlertType.PlatformUnreachable, humanMessage: 'The platform is unreachable.' },
  resourceId: '20000000-0000-0000-0000-000000000001',
  resourceName: 'Production',
  resourceType: AlertResourceType.Platform,
  resourcePath: null,
  acknowledgedByActorId: null,
  acknowledgedAt: null,
  resolvedByActorId: null,
  resolvedAt: null,
  actorId: null,
  actorName: null,
  actorType: null,
  resolutionNote: null,
  createdAt: '2026-07-28T12:00:00Z',
  updatedAt: '2026-07-28T12:00:00Z',
};

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{`${location.pathname}${location.search}${location.hash}`}</span>;
}

describe('AlertBell', () => {
  beforeEach(() => {
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})),
      http.get(`http://localhost/api/v1/alertEvents/${alertEvent.id}`, () => HttpResponse.json(alertEvent)),
    );
  });

  it('opens the alert sheet without leaving the current page', async () => {
    const { user } = renderCitadel(<AlertTestRoot showBell />, { route: '/platforms/example' });

    await user.click(screen.getByRole('button', { name: 'Open alert notifications' }));
    await user.click(await screen.findByText(alertEvent.message));

    expect(screen.getByTestId('location')).toHaveTextContent('/platforms/example');
    await waitFor(() => expect(screen.getByRole('dialog')).toBeVisible());
    expect(await screen.findByRole('heading', { name: AlertType.PlatformUnreachable })).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Copy alert link' }));
    expect(await navigator.clipboard.readText()).toBe(`${window.location.origin}/alerts/${alertEvent.id}`);

    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/platforms/example'));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('opens the alert sheet when a shared URL is loaded', async () => {
    const { user } = renderCitadel(<AlertTestRoot />, {
      route: `/alerts/${alertEvent.id}`,
    });

    expect(await screen.findByRole('heading', { name: AlertType.PlatformUnreachable })).toBeVisible();
    expect(screen.getByTestId('location')).toHaveTextContent(`/alerts/${alertEvent.id}`);

    await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(screen.getByTestId('location')).toHaveTextContent('/alerts'));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});

describe('Header', () => {
  it('places live connection status between the breadcrumb and search controls', () => {
    server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));

    renderCitadel(
      <Provider>
        <AppContext.Provider
          value={{
            isLoading: false,
            currentPlatform: undefined,
            platforms: undefined,
            unresolvedAlertCount: 0,
            liveAlertEvents: {},
            receivedAlertEventIds: [],
          }}>
          <LayoutContext.Provider
            value={{
              sidebarMinimized: false,
              mobileMenuVisible: false,
              toggleSidebar: vi.fn(),
              setSidebarOpen: vi.fn(),
              toggleMobileMenu: vi.fn(),
            }}>
            <AppearanceProvider>
              <Header />
            </AppearanceProvider>
          </LayoutContext.Provider>
        </AppContext.Provider>
      </Provider>,
    );

    const search = screen.getByTestId('global-search');
    const indicator = screen.getByTestId('live-connection-indicator');
    const breadcrumb = screen.getByTestId('breadcrumb');
    const header = screen.getByRole('banner');

    expect(breadcrumb.parentElement?.nextElementSibling).toContainElement(indicator);
    expect(indicator.parentElement?.nextElementSibling).toContainElement(search);
    expect(header).toHaveClass('border-sidebar-border', 'bg-sidebar');
  });
});

function AlertTestRoot({ showBell = false }: { showBell?: boolean }) {
  return (
    <Provider>
      <AppContext.Provider
        value={{
          isLoading: false,
          currentPlatform: undefined,
          platforms: undefined,
          unresolvedAlertCount: 1,
          liveAlertEvents: { [alertEvent.id]: alertEvent },
          receivedAlertEventIds: [],
        }}>
        {showBell && <AlertBell />}
        <AlertTaskSheet />
        <LocationProbe />
      </AppContext.Provider>
    </Provider>
  );
}

beforeEach(() => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((media: string) => ({
      matches: false,
      media,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
});
