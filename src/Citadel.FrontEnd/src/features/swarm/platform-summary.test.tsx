import { act, screen, waitFor, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { SwarmQuorumState } from '@/api/generated/api.types';
import { calculateSwarmQuorum } from './hooks/useSwarmOverview';
import { applySwarmInventoryToOverview, SwarmPlatformSummary } from './platform-summary';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('SwarmPlatformSummary', () => {
  it('keeps an early SignalR snapshot instead of allowing an older HTTP response to replace it', async () => {
    const fake = new FakeHubConnection();
    let releaseResponse!: () => void;
    const responseGate = new Promise<void>((resolve) => {
      releaseResponse = resolve;
    });
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm`, async () => {
        requestCount++;
        await responseGate;
        return HttpResponse.json({
          ...overview(),
          health: 'Stale',
          message: 'older HTTP state',
          isStale: true,
        });
      }),
    );

    renderCitadel(
      <SwarmPlatformSummary platformId={platformId} networkCount={4} serviceStatusCounts={managedServiceCounts()} />,
      {
        signalR: {
          connectionFactory: () => fake.asHubConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    await waitFor(() => expect(requestCount).toBe(1));
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() => fake.emit('SwarmInventoryUpdated', emptyInventory()));

    const summary = await screen.findByRole('region', { name: 'Swarm cluster summary' });
    await waitFor(() => expect(within(summary).getByRole('link', { name: /Managers/ })).toHaveTextContent('0'));
    const cards = within(summary).getAllByRole('link');
    expect(cards[0]).toHaveTextContent('Managers');
    expect(cards[1]).toHaveTextContent('Nodes');
    expect(cards[2]).toHaveTextContent('Services');
    expect(cards[2]).toHaveTextContent('4');
    expect(cards[2]).toHaveAttribute('href', `/swarm-services?platformId=${platformId}`);
    expect(within(cards[2]).getByLabelText('Healthy: 1')).toBeVisible();
    expect(within(cards[2]).getByLabelText('Failed: 1')).toBeVisible();
    expect(cards[3]).toHaveTextContent('Running tasks');
    expect(cards[4]).toHaveTextContent('Networks');
    expect(cards[4]).toHaveTextContent('4');
    expect(cards[4]).toHaveAttribute('href', `/platforms/${platformId}/networks`);
    expect(cards[5]).toHaveTextContent('Backups');
    expect(cards[5]).toHaveAttribute('href', '/backup-policies');
    expect(screen.queryByText('Swarm cluster')).not.toBeInTheDocument();
    expect(screen.queryByText('Healthy')).not.toBeInTheDocument();
    expect(screen.queryByText(/Last observed/)).not.toBeInTheDocument();
    expect(screen.queryByText('No inventory observed yet.')).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Nodes/ })).toHaveAttribute('href', `/platforms/${platformId}/nodes`);
    expect(screen.getByRole('status', { name: 'Swarm quorum unknown' })).toBeVisible();
    expect(screen.queryByText('older HTTP state')).not.toBeInTheDocument();

    await act(async () => {
      releaseResponse();
      await responseGate;
    });

    await waitFor(() => expect(screen.getByRole('region', { name: 'Swarm cluster summary' })).toBeVisible());
    expect(screen.queryByText('Healthy')).not.toBeInTheDocument();
    expect(screen.queryByText('older HTTP state')).not.toBeInTheDocument();
    expect(requestCount).toBe(1);
  });

  it.each(['Offline', 'Degraded'])('clears a previous %s state after a healthy inventory update', (health) => {
    const updated = applySwarmInventoryToOverview(healthyInventory(), {
      ...overview(),
      health,
      message: 'old connection state',
    });

    expect(updated.health).toBe('Healthy');
    expect(updated.message).toBeNull();
  });

  it('derives lost quorum from live manager reachability', () => {
    const inventory = healthyInventory();
    inventory.nodes.items.push(manager('manager-2', false, 'Unreachable'));
    inventory.nodes.items.push(manager('manager-3', false, 'Unreachable'));

    const updated = applySwarmInventoryToOverview(inventory, overview());

    expect(updated.quorum).toEqual({
      state: SwarmQuorumState.Lost,
      reachableManagers: 1,
      requiredManagers: 2,
      hasLeader: true,
    });
    expect(updated.health).toBe('Degraded');
    expect(updated.message).toContain('1 of 3 managers are reachable');
  });

  it('does not report stale manager inventory as lost quorum', () => {
    const nodes = [manager('manager-1', true, 'Reachable', true)];

    expect(calculateSwarmQuorum(nodes).state).toBe(SwarmQuorumState.Unknown);
  });

  it('updates service state counts from a SignalR inventory snapshot', () => {
    const inventory = emptyInventory();
    inventory.services.items = [
      service('healthy', 2, 2),
      service('degraded', 1, 2),
      service('failed', 0, 1),
      service('stopped', 0, 0),
      service('unknown', 1, 1, true),
    ];

    const updated = applySwarmInventoryToOverview(inventory, overview());

    expect(updated.serviceCount).toBe(5);
    expect(updated.serviceStatusCounts).toEqual({
      total: 5,
      healthy: 1,
      degraded: 1,
      failed: 1,
      stopped: 1,
      paused: 0,
      inProgress: 0,
      unknown: 1,
    });
  });
});

function emptyInventory(): SwarmInventoryUpdate {
  return {
    platformId,
    nodes: { items: [] },
    services: { items: [] },
    tasks: { items: [] },
    networks: { items: [] },
    secrets: { items: [] },
    configs: { items: [] },
  };
}

function healthyInventory(): SwarmInventoryUpdate {
  const inventory = emptyInventory();
  inventory.nodes.items = [manager('manager-1', true, 'Reachable')];
  return inventory;
}

function manager(id: string, isLeader: boolean, reachability: string, isStale = false) {
  return {
    id,
    role: 'Manager',
    isLeader,
    reachability,
    isStale,
  } as SwarmInventoryUpdate['nodes']['items'][number];
}

function overview() {
  return {
    platformId,
    health: 'Healthy',
    message: null,
    isStale: false,
    nodeCount: 0,
    managerCount: 0,
    quorum: {
      state: SwarmQuorumState.Unknown,
      reachableManagers: 0,
      requiredManagers: 0,
      hasLeader: false,
    },
    serviceCount: 0,
    serviceStatusCounts: {
      total: 0,
      healthy: 0,
      degraded: 0,
      failed: 0,
      stopped: 0,
      paused: 0,
      inProgress: 0,
      unknown: 0,
    },
    runningTaskCount: 0,
    desiredTaskCount: 0,
    networkCount: 0,
  };
}

function managedServiceCounts() {
  return {
    total: 4,
    healthy: 1,
    degraded: 1,
    failed: 1,
    stopped: 0,
    paused: 0,
    inProgress: 1,
    unknown: 0,
  };
}

function service(id: string, runningTaskCount: number, desiredTaskCount: number, isStale = false) {
  return {
    id,
    runningTaskCount,
    desiredTaskCount,
    updateState: 'Completed',
    isStale,
  } as SwarmInventoryUpdate['services']['items'][number];
}
