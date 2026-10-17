import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { CheckConfiguration, HealthStatus, PlatformStatus, type HealthMember } from '@/api/generated/api.types';
import { createSwarmTask, createWorkloadHealth } from '@/test/factories/resources';
import { WorkloadServiceHealthCard, serviceHealthMembers } from './service-card';
import { WorkloadContainerHealthCard } from './card';

const task = createSwarmTask({ id: 'task', serviceId: 'service', nodeId: 'node', desiredState: 'Running' });
const service = { id: 'service', name: 'web', desiredTaskCount: 1, isStale: false, tasks: [task] };
const member: HealthMember = {
  platformId: 'platform',
  nodeId: 'node',
  taskId: 'task',
  service: 'service',
  dockerContainerId: 'docker-id',
  containerId: 'database-id',
  name: 'web.1.task',
  available: true,
  state: 'Running',
  status: HealthStatus.Healthy,
  observation: { observedAt: Date.now(), configuration: CheckConfiguration.Enabled },
};
const resource = () => ({
  id: 'stack',
  name: 'web',
  platformStatus: PlatformStatus.Online,
  probeHealth: createWorkloadHealth({ evaluatedAt: Date.now(), members: [member] }),
});

describe('runtime table health', () => {
  it('matches the exact container and node and follows updated observations', () => {
    const container = { id: 'docker-id', name: 'web', platformId: 'platform', dockerNodeId: 'node' };
    const view = render(
      <MemoryRouter>
        <WorkloadContainerHealthCard resource={resource()} container={container} />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Healthy' })).toBeVisible();
    const changed = resource();
    changed.probeHealth.members = [{ ...member, status: HealthStatus.Unhealthy }];
    view.rerender(
      <MemoryRouter>
        <WorkloadContainerHealthCard resource={changed} container={container} />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Unhealthy' })).toBeVisible();
    view.rerender(
      <MemoryRouter>
        <WorkloadContainerHealthCard resource={changed} container={{ ...container, dockerNodeId: 'other' }} />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Unknown' })).toBeVisible();
  });

  it('excludes historical tasks, other nodes, platforms, and services', () => {
    const owner = resource();
    owner.probeHealth.members = [
      member,
      { ...member, taskId: 'old', status: HealthStatus.Unhealthy },
      { ...member, nodeId: 'other' },
      { ...member, platformId: 'other' },
      { ...member, service: 'other' },
    ];
    const withHistory = { ...service, tasks: [task, { ...task, id: 'old', desiredState: 'Shutdown' }] };
    expect(serviceHealthMembers(owner, withHistory, 'platform')).toEqual([member]);
    render(
      <MemoryRouter>
        <WorkloadServiceHealthCard resource={owner} service={withHistory} platformId="platform" />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Healthy' })).toBeVisible();
  });

  it('shows incomplete replica coverage and treats stale tasks as unknown', () => {
    const view = render(
      <MemoryRouter>
        <WorkloadServiceHealthCard
          resource={resource()}
          service={{ ...service, desiredTaskCount: 2 }}
          platformId="platform"
        />
      </MemoryRouter>,
    );
    const trigger = screen.getByRole('button', { name: 'Health: Partial health' });
    fireEvent.click(trigger);
    expect(screen.getByText('1 observed · 2 expected current tasks')).toBeVisible();
    expect(screen.getByText('Unknown')).toBeVisible();
    view.rerender(
      <MemoryRouter>
        <WorkloadServiceHealthCard
          resource={resource()}
          service={{ ...service, tasks: [{ ...task, isStale: true }] }}
          platformId="platform"
        />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Unknown' })).toBeVisible();
  });

  it('does not report passing checks when the platform is offline', () => {
    render(
      <MemoryRouter>
        <WorkloadServiceHealthCard
          resource={{ ...resource(), platformStatus: PlatformStatus.Offline }}
          service={service}
          platformId="platform"
        />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button', { name: 'Health: Unknown' })).toBeVisible();
  });
});
