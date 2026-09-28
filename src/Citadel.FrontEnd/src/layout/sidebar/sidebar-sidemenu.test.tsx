import { SidebarProvider } from '@/components/ui/sidebar';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ISubMenuItem } from './menu-items';
import { SidebarSubMenu } from './sidebar-sidemenu';

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
    <SidebarProvider defaultOpen={!sidebarMinimized}>
      <SidebarSubMenu submenu={submenu} toggleMenu={vi.fn()} />
    </SidebarProvider>,
  );

describe('SidebarSubMenu', () => {
  beforeEach(() => {
    vi.mocked(window.matchMedia).mockImplementation(
      (media) =>
        ({
          matches: false,
          media,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as MediaQueryList,
    );
  });
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

  it('keeps expanded mobile links available when the desktop sidebar was minimized', () => {
    vi.mocked(window.matchMedia).mockImplementation(
      (media) =>
        ({
          matches: media === '(max-width: 1023px)',
          media,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as MediaQueryList,
    );
    renderSubMenu({ ...settingsMenu, expanded: true }, true);

    expect(screen.getByRole('link', { name: 'Tags' })).toHaveAttribute('href', '/tags');
  });
});
