import { SidebarProvider } from '@/components/ui/sidebar';
import { renderCitadel } from '@/test/render-citadel';
import userEvent from '@testing-library/user-event';
import { screen } from '@testing-library/react';
import { UpdateNotice } from '../update-notice';
import { Sidebar } from './sidebar';

const mocks = vi.hoisted(() => ({
  isAdministrator: false,
  availableUpdate: undefined as { version: string; releaseUrl: string } | undefined,
}));

vi.mock('@/lib/context/layout-context', () => ({
  useLayoutContext: () => ({ sidebarMinimized: false }),
}));

vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ applicationInfo: { version: '1.0.0', availableUpdate: mocks.availableUpdate } }),
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

const renderSidebar = (open = true) =>
  renderCitadel(
    <SidebarProvider defaultOpen={open}>
      <Sidebar />
    </SidebarProvider>,
  );

describe('Sidebar license link', () => {
  beforeEach(() => {
    mocks.isAdministrator = false;
    mocks.availableUpdate = undefined;
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

  it.each([true, false])('opens release details from the version indicator (expanded: %s)', async (open) => {
    mocks.isAdministrator = true;
    mocks.availableUpdate = {
      version: '1.1.0',
      releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0',
    };
    renderSidebar(open);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Citadel 1.1.0 available' }));
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(screen.getByRole('link', { name: 'Release notes' })).toHaveAttribute(
      'href',
      mocks.availableUpdate.releaseUrl,
    );
    expect(screen.getByRole('link', { name: 'Upgrade guide' })).toHaveAttribute(
      'href',
      'https://docs.citadelplane.com/docs/operations/upgrade-and-rollback/',
    );
  });

  it('hides update controls from non-administrators', () => {
    mocks.availableUpdate = {
      version: '1.1.0',
      releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0',
    };
    renderSidebar();
    renderCitadel(<UpdateNotice />);
    expect(screen.queryByRole('button', { name: 'Citadel 1.1.0 available' })).not.toBeInTheDocument();
    expect(screen.queryByText('New version available')).not.toBeInTheDocument();
  });

  it('shows the Home notice only when an update is available', () => {
    mocks.isAdministrator = true;
    const { unmount } = renderCitadel(<UpdateNotice />);
    expect(screen.queryByText('New version available')).not.toBeInTheDocument();
    unmount();
    mocks.availableUpdate = {
      version: '1.1.0',
      releaseUrl: 'https://github.com/Citadel-P/Citadel/releases/tag/v1.1.0',
    };
    renderCitadel(<UpdateNotice />);
    expect(screen.getByText('New version available')).toBeVisible();
    expect(screen.getByRole('button', { name: 'View update' })).toBeVisible();
  });
});
