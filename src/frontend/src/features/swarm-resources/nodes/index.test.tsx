import { SwarmNodeView, SwarmTaskView } from '@/api/generated/api.types';
import { RegularResourceView } from '@/pages/regular-resource';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { NodeComponents } from '.';
import { LayoutContext } from '@/lib/context/layout-context';

vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

const platformId = '00000000-0000-0000-0000-000000000200';
const layoutContext = {
  theme: { mode: 'light' as const },
  sidebarMinimized: false,
  mobileMenuVisible: false,
  toggleSidebar: vi.fn(),
  setSidebarOpen: vi.fn(),
  toggleMobileMenu: vi.fn(),
  toggleThemeColor: vi.fn(),
  setThemeMode: vi.fn(),
};

describe('NodeComponents', () => {
  it('uses expandable task rows without duplicating the indicated node status', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/nodes`, () =>
        HttpResponse.json({ items: [node({ availability: 'Pause' })] }),
      ),
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({ items: [task()] }),
      ),
    );

    const { user } = renderCitadel(
      <LayoutContext.Provider value={layoutContext}>
        <Routes>
          <Route
            path="/platforms/:platformId/nodes"
            element={<RegularResourceView Components={NodeComponents} type="Node" showTaskSheet={false} />}
          />
        </Routes>
      </LayoutContext.Provider>,
      {
        route: `/platforms/${platformId}/nodes`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    const nodeLink = await screen.findByRole('link', { name: 'manager-1' });
    expect(nodeLink).toHaveAttribute('href', `/platforms/${platformId}/nodes/node-1`);
    expect(nodeLink.parentElement?.querySelector('.bg-orange-500')).not.toBeNull();
    expect(screen.queryByRole('columnheader', { name: 'Status' })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'web.1' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('checkbox', { name: 'Select Node manager-1' }));
    expect(screen.getByText('1 of 1 node(s) selected.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Active' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Drain' })).toBeEnabled();

    await user.click(screen.getByRole('button', { name: 'Expand node manager-1' }));

    expect(screen.getByRole('link', { name: 'web.1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/tasks/task-1`,
    );
    expect(screen.getByRole('link', { name: 'web' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/services/service-1`,
    );
  });
});

const node = (overrides: Partial<SwarmNodeView> = {}): SwarmNodeView => ({
  id: 'node-1',
  versionIndex: 1,
  hostname: 'manager-1',
  role: 'Manager',
  isLeader: true,
  reachability: 'Reachable',
  status: 'Ready',
  statusMessage: null,
  availability: 'Active',
  engineVersion: '29.0',
  operatingSystem: 'linux',
  architecture: 'x86_64',
  address: '10.0.0.1',
  labels: {},
  runningTaskCount: 1,
  desiredTaskCount: 1,
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T08:00:00Z',
  isStale: false,
  capabilities: {
    canRead: true,
    canWrite: true,
    canExecute: false,
    canViewLogs: false,
    canInspect: true,
    canOpenTerminal: false,
    canManageNodeAgents: false,
    canPull: false,
  },
  ...overrides,
});

const task = (overrides: Partial<SwarmTaskView> = {}): SwarmTaskView => ({
  id: 'task-1',
  versionIndex: 1,
  name: 'web.1',
  serviceId: 'service-1',
  serviceName: 'web',
  slot: 1,
  nodeId: 'node-1',
  nodeHostname: 'manager-1',
  desiredState: 'Running',
  state: 'Running',
  statusMessage: null,
  error: null,
  image: 'nginx:latest',
  ports: [],
  statusTimestamp: '2026-08-05T08:00:00Z',
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T08:00:00Z',
  isStale: false,
  capabilities: null,
  ...overrides,
});
