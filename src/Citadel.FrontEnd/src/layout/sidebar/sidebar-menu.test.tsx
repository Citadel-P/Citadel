import { screen } from '@testing-library/react';
import { Link, useLocation } from 'react-router';
import { PlatformType } from '@/api/generated/api.types';
import { SidebarProvider } from '@/components/ui/sidebar';
import { renderCitadel } from '@/test/render-citadel';
import { SidebarMenu } from './sidebar-menu';

const mocks = vi.hoisted(() => ({
  administrator: true,
  platforms: [] as { id: string; name: string; type: string }[],
}));
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ platforms: mocks.platforms, unresolvedAlertCount: 3 }),
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: () => ({ data: { data: { authorization: { isAdministrator: mocks.administrator } } } }),
}));

function Harness() {
  const location = useLocation();
  return (
    <SidebarProvider>
      <SidebarMenu />
      <output data-testid="location">{location.pathname}</output>
      <Link to="/license">Open license page</Link>
    </SidebarProvider>
  );
}

beforeEach(() => {
  localStorage.clear();
  mocks.administrator = true;
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

it('opens the active settings group on navigation and retains access filtering', async () => {
  const { user } = renderCitadel(<Harness />, { route: '/deployments' });
  expect(screen.getByRole('button', { name: 'Settings' })).toHaveAttribute('aria-expanded', 'false');
  await user.click(screen.getByRole('link', { name: 'Open license page' }));
  const settings = screen.getByRole('button', { name: 'Settings' });
  expect(settings).toHaveAttribute('aria-expanded', 'true');
  expect(screen.getByRole('link', { name: 'License' })).toHaveAttribute('aria-current', 'page');
  await user.click(settings);
  expect(settings).toHaveAttribute('aria-expanded', 'false');
});

it('does not expose administrator links or removed platforms', () => {
  mocks.administrator = false;
  mocks.platforms = [];
  renderCitadel(<Harness />, { route: '/platforms/one/containers' });
  expect(screen.queryByRole('button', { name: 'Settings' })).not.toBeInTheDocument();
  expect(screen.queryByRole('link', { name: 'Containers' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Select platform' })).toBeVisible();
});
