import { LayoutContext } from '@/lib/context/layout-context';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ISubMenuItem } from './menu-items';
import { SidebarSubMenu } from './sidebar-sidemenu';

const layoutValue = {
  theme: { mode: 'light' as const, color: 'blue' },
  sidebarMinimized: false,
  mobileMenuVisible: false,
  toggleSidebar: vi.fn(),
  setSidebarOpen: vi.fn(),
  toggleMobileMenu: vi.fn(),
  toggleThemeColor: vi.fn(),
  setThemeMode: vi.fn(),
};

const settingsMenu: ISubMenuItem = {
  label: 'Settings',
  route: '/settings',
  children: [
    {
      label: 'Tags',
      route: '/tags',
    },
  ],
};

const renderSubMenu = (submenu: ISubMenuItem, sidebarMinimized = false) =>
  renderCitadel(
    <LayoutContext.Provider value={{ ...layoutValue, sidebarMinimized }}>
      <SidebarSubMenu submenu={submenu} toggleMenu={vi.fn()} />
    </LayoutContext.Provider>,
  );

describe('SidebarSubMenu', () => {
  it('removes collapsed submenu links from the layout and interaction tree', () => {
    const { container } = renderSubMenu({ ...settingsMenu, expanded: false });

    expect(screen.queryByRole('link', { name: 'Tags' })).not.toBeInTheDocument();
    expect(container.querySelector('[data-slot="sidebar-menu-sub"]')).not.toBeInTheDocument();
  });

  it('renders links only while the submenu is expanded', () => {
    renderSubMenu({ ...settingsMenu, expanded: true });

    expect(screen.getByRole('link', { name: 'Tags' })).toHaveAttribute('href', '/tags');
  });

  it('removes submenu links when the sidebar is minimized', () => {
    const { container } = renderSubMenu({ ...settingsMenu, expanded: true }, true);

    expect(screen.queryByRole('link', { name: 'Tags' })).not.toBeInTheDocument();
    expect(container.querySelector('[data-slot="sidebar-menu-sub"]')).not.toBeInTheDocument();
  });
});
