import { screen } from '@testing-library/react';
import { Link, useLocation } from 'react-router';
import { PlatformType } from '@/api/generated/api.types';
import { Sidebar as SidebarRoot, SidebarProvider, useSidebar } from '@/components/ui/sidebar';
import { renderCitadel } from '@/test/render-citadel';
import { SidebarMenu } from './sidebar-menu';
import { SidebarUtilities } from './sidebar-utilities';

const mocks = vi.hoisted(() => ({
  administrator: true,
  configuration: false,
  bindings: true,
  platforms: [] as { id: string; name: string; type: string }[],
}));
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ platforms: mocks.platforms, unresolvedAlertCount: 3 }),
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: () => ({
    data: {
      data: {
        authorization: {
          isAdministrator: mocks.administrator,
          alertRules: { canRead: mocks.configuration },
          bindings: { canRead: mocks.configuration && mocks.bindings },
          tags: { canRead: mocks.configuration },
        },
      },
    },
  }),
}));

function SidebarState() {
  const { state, openMobile, setOpenMobile } = useSidebar();
  return (
    <>
      <button onClick={() => setOpenMobile(true)}>Open navigation</button>
      <output data-testid="sidebar-state">
        {state}:{String(openMobile)}
      </output>
    </>
  );
}

function Harness({ open = true, drawer = false }: { open?: boolean; drawer?: boolean }) {
  const location = useLocation();
  return (
    <SidebarProvider defaultOpen={open}>
      {drawer ? (
        <SidebarRoot>
          <SidebarMenu />
          <SidebarUtilities />
        </SidebarRoot>
      ) : (
        <>
          <SidebarMenu />
          <SidebarUtilities />
        </>
      )}
      <SidebarState />
      <output data-testid="location">{location.pathname}</output>
      <Link to="/license">Open license page</Link>
    </SidebarProvider>
  );
}

beforeEach(() => {
  localStorage.clear();
  mocks.administrator = true;
  mocks.configuration = false;
  mocks.bindings = true;
  mocks.platforms = [
    { id: 'one', name: 'Production', type: PlatformType.Docker },
    { id: 'two', name: 'Staging', type: PlatformType.DockerSwarm },
  ];
  vi.mocked(window.matchMedia).mockImplementation(
    (media) =>
      ({ matches: false, media, addEventListener: vi.fn(), removeEventListener: vi.fn() }) as unknown as MediaQueryList,
  );
});

it('keeps Containers selected on a container detail page and switches platform without accumulating menus', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/platforms/one/containers/container-id' });
  expect(screen.getByRole('link', { name: 'Containers' })).toHaveAttribute('aria-current', 'page');
  await user.click(screen.getByRole('button', { name: 'Switch platform: Production' }));
  await user.click(screen.getByRole('menuitem', { name: 'Staging' }));
  expect(screen.getByTestId('location')).toHaveTextContent('/platforms/two/containers');
  expect(screen.getByRole('button', { name: 'Switch platform: Staging' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Switch platform: Production' })).not.toBeInTheDocument();
  expect(screen.getAllByRole('link', { name: 'Containers' })).toHaveLength(1);
  expect(screen.getByRole('link', { name: 'Nodes' })).toHaveAttribute('href', '/platforms/two/nodes');
});

it('falls back to platform overview when the target does not support the current resource', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/platforms/two/services/service-id' });
  await user.click(screen.getByRole('button', { name: 'Switch platform: Staging' }));
  await user.click(screen.getByRole('menuitem', { name: 'Production' }));
  expect(screen.getByTestId('location')).toHaveTextContent('/platforms/edit/one');
  expect(screen.getByRole('link', { name: 'Overview' })).toHaveAttribute('aria-current', 'page');
  expect(screen.queryByRole('link', { name: 'Services' })).not.toBeInTheDocument();
});

it('highlights the resource family on edit pages', () => {
  renderCitadel(<Harness />, { route: '/deployments/edit/deployment-id' });
  expect(screen.getByRole('link', { name: 'Deployments' })).toHaveAttribute('aria-current', 'page');
  expect(screen.getByRole('link', { name: 'Platforms' })).not.toHaveAttribute('aria-current');
});

it('retains the selected platform and its collapse state on global pages', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/platforms/one/containers/container-id' });
  await user.click(screen.getByRole('button', { name: 'Platform resources' }));
  expect(screen.queryByRole('link', { name: 'Containers' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('link', { name: 'Deployments' }));
  expect(screen.getByRole('button', { name: 'Switch platform: Production' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Platform resources' })).toHaveAttribute('aria-expanded', 'false');
  await user.click(screen.getByRole('button', { name: 'Platform resources' }));
  expect(screen.getByRole('link', { name: 'Containers' })).toHaveAttribute('href', '/platforms/one/containers');
  expect(screen.getByRole('link', { name: 'Containers' })).not.toHaveAttribute('aria-current');
  expect(screen.getByRole('link', { name: 'Deployments' })).toHaveAttribute('aria-current', 'page');
});

it('restores the platform and expansion preference after a reload', () => {
  localStorage.setItem('sidebar-platform-id', JSON.stringify('two'));
  localStorage.setItem('sidebar-platform-expanded', 'false');
  renderCitadel(<Harness />, { route: '/deployments' });
  expect(screen.getByRole('button', { name: 'Switch platform: Staging' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Platform resources' })).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByRole('link', { name: 'Containers' })).not.toBeInTheDocument();
});

it('highlights Administration without opening its popup on navigation', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/deployments' });
  const administration = screen.getByRole('button', { name: 'Administration' });
  expect(administration).toHaveAttribute('aria-expanded', 'false');
  expect(administration).toHaveAttribute('data-active', 'false');
  await user.click(screen.getByRole('link', { name: 'Open license page' }));
  expect(administration).toHaveAttribute('data-active', 'true');
  expect(administration).toHaveAttribute('aria-expanded', 'false');
  await user.click(administration);
  expect(screen.getByRole('menuitem', { name: 'License' })).toHaveAttribute('aria-current', 'page');
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(administration).toHaveFocus();
});

it('does not expose administrator links or removed platforms', () => {
  mocks.administrator = false;
  mocks.platforms = [];
  renderCitadel(<Harness />, { route: '/platforms/one/containers' });
  expect(screen.queryByRole('button', { name: 'Administration' })).not.toBeInTheDocument();
  expect(screen.queryByRole('link', { name: 'Containers' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Select platform' })).toBeVisible();
});

it('shows Configuration independently of Administration for permitted non-administrators', async () => {
  mocks.administrator = false;
  mocks.configuration = true;
  const { user } = renderCitadel(<Harness />, { route: '/tags' });
  const configuration = screen.getByRole('button', { name: 'Configuration' });
  expect(configuration).toHaveAttribute('data-active', 'true');
  expect(configuration).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByRole('button', { name: 'Administration' })).not.toBeInTheDocument();
  await user.click(configuration);
  expect(screen.getByRole('menuitem', { name: 'Tags' })).toHaveAttribute('aria-current', 'page');
  expect(screen.getByRole('menuitem', { name: 'Bindings' })).toHaveAttribute('href', '/bindings');
  expect(screen.queryByRole('menuitem', { name: 'Instance' })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  expect(configuration).toHaveAttribute('aria-expanded', 'false');
});

it('opens Instance from the collapsed sidebar without expanding it and hides empty Configuration', async () => {
  const { user } = renderCitadel(<Harness open={false} />, { route: '/instance' });
  const administration = screen.getByRole('button', { name: 'Administration' });
  expect(administration).toHaveAttribute('data-active', 'true');
  expect(screen.queryByRole('button', { name: 'Configuration' })).not.toBeInTheDocument();
  await user.click(administration);
  expect(screen.getByRole('menuitem', { name: 'Instance' })).toHaveAttribute('aria-current', 'page');
  expect(screen.getByTestId('sidebar-state')).toHaveTextContent('collapsed');
  await user.click(screen.getByRole('menuitem', { name: 'Updates' }));
  expect(screen.getByTestId('location')).toHaveTextContent('/updates');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(screen.getByTestId('sidebar-state')).toHaveTextContent('collapsed');
});

it('keeps All platforms available from the platform browser', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/deployments' });
  await user.click(screen.getByRole('button', { name: 'Select platform' }));
  await user.click(screen.getByRole('menuitem', { name: 'All platforms' }));
  expect(screen.getByTestId('location')).toHaveTextContent('/');
});

it('supports keyboard selection in utility menus', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/deployments' });
  screen.getByRole('button', { name: 'Administration' }).focus();
  await user.keyboard('{Enter}');
  expect(screen.getByRole('menuitem', { name: 'Instance' })).toHaveFocus();
  await user.keyboard('{ArrowDown}{Enter}');
  expect(screen.getByTestId('location')).toHaveTextContent('/access');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});

it('filters individual Configuration destinations by permission', async () => {
  mocks.configuration = true;
  mocks.bindings = false;
  const { user } = renderCitadel(<Harness />);
  await user.click(screen.getByRole('button', { name: 'Configuration' }));
  expect(screen.getByRole('menuitem', { name: 'Tags' })).toBeVisible();
  expect(screen.queryByRole('menuitem', { name: 'Bindings' })).not.toBeInTheDocument();
});

it('opens utility menus inside mobile navigation and closes the drawer after selection', async () => {
  vi.mocked(window.matchMedia).mockImplementation(
    (media) =>
      ({
        matches: media === '(max-width: 1023px)',
        media,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }) as unknown as MediaQueryList,
  );
  const { user } = renderCitadel(<Harness drawer />, { route: '/deployments' });
  await user.click(screen.getByRole('button', { name: 'Open navigation' }));
  expect(screen.getByRole('dialog')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Administration' }));
  expect(screen.getByRole('menuitem', { name: 'Instance' })).toBeVisible();
  await user.click(screen.getByRole('menuitem', { name: 'Instance' }));
  expect(screen.getByTestId('location')).toHaveTextContent('/instance');
  expect(screen.getByTestId('sidebar-state')).toHaveTextContent('expanded:false');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
