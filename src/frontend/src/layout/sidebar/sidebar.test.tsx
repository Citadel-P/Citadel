import { SidebarProvider } from '@/components/ui/sidebar';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { Sidebar } from './sidebar';

const mocks = vi.hoisted(() => ({ isAdministrator: false }));

vi.mock('@/lib/context/layout-context', () => ({
  useLayoutContext: () => ({ sidebarMinimized: false }),
}));

vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ applicationInfo: { version: '1.0.0' } }),
}));

vi.mock('@/lib/hooks', () => ({
  useRead: (resource: string) => ({
    data:
      resource === 'getCurrentProfile'
        ? { data: { authorization: { isAdministrator: mocks.isAdministrator } } }
        : { data: { version: '1.0.0' } },
  }),
}));

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ entitlements: { effectiveEdition: 'Team' } }),
}));

vi.mock('./sidebar-menu', () => ({ SidebarMenu: () => null }));

const renderSidebar = () =>
  renderCitadel(
    <SidebarProvider>
      <Sidebar />
    </SidebarProvider>,
  );

describe('Sidebar license link', () => {
  beforeEach(() => {
    vi.mocked(window.matchMedia).mockImplementation(
      (query: string): MediaQueryList => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      }),
    );
  });

  it('shows the license edition without a link to non-administrators', () => {
    mocks.isAdministrator = false;

    renderSidebar();

    expect(screen.getByText('Team')).toBeVisible();
    expect(screen.queryByRole('link', { name: 'Team' })).not.toBeInTheDocument();
  });

  it('shows the license link to administrators', () => {
    mocks.isAdministrator = true;

    renderSidebar();

    expect(screen.getByRole('link', { name: 'Team' })).toHaveAttribute('href', '/license');
  });
});
