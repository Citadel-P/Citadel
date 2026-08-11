import {
  ContainerStateStatus,
  PlatformType,
  ResourceControlState,
  StackDriftMode,
  StackReleaseStatus,
  StackView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { StackFormComponents } from '.';

const { useReadMock, useServicesGroupMock, useStackInfoGroupMock } = vi.hoisted(() => ({
  useReadMock: vi.fn(() => ({ data: undefined, error: undefined })),
  useServicesGroupMock: vi.fn(() => ({ items: [], isLoading: false })),
  useStackInfoGroupMock: vi.fn(() => ({ containersInfo: [], isLoading: false, error: undefined })),
}));

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: useReadMock,
}));
vi.mock('@/features/swarm-resources/services/hooks/useServicesGroup', () => ({
  useServicesGroup: useServicesGroupMock,
}));
vi.mock('./hooks/useStackInfoGroup', () => ({
  useStackInfoGroup: useStackInfoGroupMock,
}));
vi.mock('@/features/swarm-resources/services/table', () => ({
  ServicesTable: ({
    items,
    taskContainers,
  }: {
    items: Array<{ id: string; name: string }>;
    taskContainers?: unknown[];
  }) => (
    <>
      <ul>
        {items.map((item) => (
          <li key={item.id}>{item.name}</li>
        ))}
      </ul>
      {taskContainers && <div>Runtime stats for {taskContainers.length} containers</div>}
    </>
  ),
}));
vi.mock('@/features/swarm-resources/services/service-info/inspect', () => ({
  ServiceInspect: ({ service }: { service: { id: string } }) => <div>Inspecting {service.id}</div>,
}));
vi.mock('@/features/swarm-services/form/service-terminal', () => ({
  ServiceTerminal: ({ tasks }: { tasks: unknown[] }) => <div>Terminal tasks: {tasks.length}</div>,
}));

vi.mock('@/lib/monaco', () => ({
  MonacoDiff: () => null,
  MonacoEditor: () => null,
  MonacoToArrayEditor: () => null,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-logs', () => ({
  StackLogs: ({ containers }: { containers: string[] }) => <div>Logs for {containers.join(', ')}</div>,
}));
vi.mock('@/features/docker-resources/containers/container-info/stack-stats', () => ({
  StackStats: ({ containers }: { containers: unknown[] }) => <div>Stats for {containers.length} containers</div>,
}));
vi.mock('monaco-editor', () => ({ MarkerSeverity: { Warning: 4 } }));

const stack = (platformType: PlatformType): StackView =>
  ({
    id: '019f0000-0000-7000-8000-000000000001',
    platformType,
    status: StackReleaseStatus.Created,
    driftPolicy: { mode: StackDriftMode.Disabled },
  }) as StackView;

describe('Stack subheader', () => {
  beforeEach(() => useReadMock.mockClear());

  it('does not show Standalone drift guidance for a Swarm Stack', () => {
    const SubHeader = StackFormComponents.EditForm!.SubHeader!;

    renderCitadel(<SubHeader resource={stack(PlatformType.DockerSwarm)} />);

    expect(screen.queryByText('Drift detection disabled')).not.toBeInTheDocument();
    expect(useReadMock).toHaveBeenCalledWith(
      'getStackDrift',
      { stackId: '019f0000-0000-7000-8000-000000000001' },
      { enabled: false },
    );
  });

  it('keeps the drift guidance for a Standalone Stack', () => {
    const SubHeader = StackFormComponents.EditForm!.SubHeader!;

    renderCitadel(<SubHeader resource={stack(PlatformType.Docker)} />);

    expect(screen.getByText('Drift detection disabled')).toBeInTheDocument();
  });
});

describe('Stack tabs', () => {
  const servicesTab = StackFormComponents.EditForm!.Tabs!.find((tab) => tab.label === 'Services')!;

  beforeEach(() => {
    useServicesGroupMock.mockReturnValue({ items: [], isLoading: false });
    useStackInfoGroupMock.mockReturnValue({ containersInfo: [], isLoading: false, error: undefined });
  });

  it('enables Services for an applied Swarm Stack', () => {
    const resource = {
      ...stack(PlatformType.DockerSwarm),
      status: StackReleaseStatus.Healthy,
    } as StackView;

    expect(servicesTab.disabled?.(resource)).toBe(false);
  });

  it('keeps Services disabled for an unapplied draft', () => {
    expect(servicesTab.disabled?.(stack(PlatformType.DockerSwarm))).toBe(true);
  });

  it('shows only Services owned by the current Stack', () => {
    const resource = {
      ...stack(PlatformType.DockerSwarm),
      platformId: '019f0000-0000-7000-8000-000000000010',
      status: StackReleaseStatus.Healthy,
    } as StackView;
    useServicesGroupMock.mockReturnValue({
      isLoading: false,
      items: [
        { id: 'owned', name: 'redis-test_web', labels: { 'com.citadel.stack-id': resource.id } },
        {
          id: 'other',
          name: 'another-stack_api',
          labels: { 'com.citadel.stack-id': '019f0000-0000-7000-8000-000000000099' },
        },
      ],
    });
    const Content = servicesTab.Content;

    renderCitadel(<Content resource={resource} />);

    expect(screen.getAllByText('redis-test_web')).not.toHaveLength(0);
    expect(screen.queryByText('another-stack_api')).not.toBeInTheDocument();
  });

  it('uses the standard Stack runtime controls for Swarm Services', async () => {
    const resource = {
      ...stack(PlatformType.DockerSwarm),
      platformId: '019f0000-0000-7000-8000-000000000010',
      status: StackReleaseStatus.Healthy,
    } as StackView;
    useServicesGroupMock.mockReturnValue({
      isLoading: false,
      items: [
        {
          id: 'web',
          name: 'redis-test_web',
          labels: { 'com.citadel.stack-id': resource.id },
          capabilities: { canViewLogs: true, canInspect: true },
          runningTaskCount: 1,
          tasks: [{ id: 'web-task' }],
        },
        {
          id: 'worker',
          name: 'redis-test_worker',
          labels: { 'com.citadel.stack-id': resource.id },
          capabilities: { canViewLogs: true, canInspect: true },
          runningTaskCount: 1,
          tasks: [{ id: 'worker-task' }],
        },
      ],
    });
    useStackInfoGroupMock.mockReturnValue({
      isLoading: false,
      error: undefined,
      containersInfo: [
        {
          id: 'container-web',
          name: '/redis-test_web.1.task-id',
          isSwarmTask: true,
        },
        {
          id: 'container-worker',
          name: '/redis-test_worker.1.task-id',
          isSwarmTask: true,
        },
      ],
    });
    const Content = servicesTab.Content;

    const { user } = renderCitadel(<Content resource={resource} />);

    expect(screen.getByRole('tab', { name: 'Stats' })).toBeVisible();
    expect(screen.getByText('Runtime stats for 2 containers')).toBeVisible();
    expect(screen.getByText('Logs for redis-test_web.1.task-id, redis-test_worker.1.task-id')).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Service' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: 'Terminal' }));
    expect(screen.getByText('Terminal tasks: 2')).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Service' })).not.toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: 'Inspect' }));
    expect(screen.getByText('Inspecting web')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Service filter' }));
    await user.click(screen.getByRole('button', { name: 'redis-test_worker' }));
    expect(screen.getByText('Inspecting worker')).toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Stats' }));
    expect(screen.getByText('Stats for 2 containers')).toBeVisible();
  });

  it('shows linked Compose containers until the first Swarm Apply creates Services', () => {
    const resource = {
      ...stack(PlatformType.DockerSwarm),
      platformId: '019f0000-0000-7000-8000-000000000010',
      status: StackReleaseStatus.Healthy,
    } as StackView;
    useStackInfoGroupMock.mockReturnValue({
      isLoading: false,
      error: undefined,
      containersInfo: [
        {
          id: '9964fd452f13',
          name: '/beszel',
          image: 'henrygd/beszel:latest',
          imageId: 'sha256:beszel',
          state: ContainerStateStatus.Running,
          controlState: ResourceControlState.Managed,
          isSystem: false,
          systemRole: null,
          hasCitadelOwnershipLabels: true,
          isSwarmTask: false,
          stackId: resource.id,
        },
      ],
    });
    const Content = servicesTab.Content;

    renderCitadel(<Content resource={resource} />);

    expect(screen.getByText('Awaiting first Swarm Apply')).toBeInTheDocument();
    expect(screen.getByText('beszel')).toBeInTheDocument();
  });

  it('does not present Swarm task containers as a pending Compose project', () => {
    const resource = {
      ...stack(PlatformType.DockerSwarm),
      platformId: '019f0000-0000-7000-8000-000000000010',
      status: StackReleaseStatus.Healthy,
    } as StackView;
    useStackInfoGroupMock.mockReturnValue({
      isLoading: false,
      error: undefined,
      containersInfo: [
        {
          id: 'task-container',
          name: '/redis-test_web.1.task',
          image: 'redis:latest',
          imageId: 'sha256:redis',
          state: ContainerStateStatus.Running,
          controlState: ResourceControlState.Managed,
          isSystem: false,
          systemRole: null,
          hasCitadelOwnershipLabels: true,
          isSwarmTask: true,
          stackId: resource.id,
        },
      ],
    });
    const Content = servicesTab.Content;

    renderCitadel(<Content resource={resource} />);

    expect(screen.queryByText('Awaiting first Swarm Apply')).not.toBeInTheDocument();
  });
});
