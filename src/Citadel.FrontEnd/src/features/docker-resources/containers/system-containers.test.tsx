import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import {
  ContainerStateStatus,
  ContainerSystemRole,
  ContainerView,
  PlatformType,
  PlatformView,
  ResourceControlState,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import {
  ContainerDropdownActions,
  ContainerGroupActions,
  type ContainerStackGroupResource,
  containsSystemContainer,
  createContainerActions,
  isAdoptableContainer,
  isContainerAdoptionAvailable,
  isImportableStack,
  isImportableStackSelection,
} from './actions';
import { SystemContainerBadge } from './system-container-badge';
import { isUnmanagedContainer } from '@/lib/utils';
import { AppContext } from '@/lib/context/app-context';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import type { ReactNode } from 'react';
import { ContainersTable } from './table';
import { DockerContainerCell } from '@/components/custom/common';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
}));
vi.mock('monaco-editor', () => ({ MarkerSeverity: { Warning: 4 } }));

const ContainerActionsContext = ({
  children,
  platformType = PlatformType.Docker,
}: {
  children: ReactNode;
  platformType?: PlatformType;
}) => (
  <AppContext.Provider
    value={{
      isLoading: false,
      currentPlatform: { id: 'platform-id', type: platformType } as PlatformView,
      platforms: [],
      unresolvedAlertCount: 0,
      liveAlertEvents: {},
      receivedAlertEventIds: [],
    }}>
    <MemoryRouter>{children}</MemoryRouter>
  </AppContext.Provider>
);

const renderActions = (children: ReactNode, platformType = PlatformType.Docker) =>
  render(<ContainerActionsContext platformType={platformType}>{children}</ContainerActionsContext>);

type TestContainer = {
  name: string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
  isSystem: boolean;
  isSwarmTask?: boolean;
};

const normalContainer: TestContainer = {
  name: 'app',
  state: ContainerStateStatus.Running,
  controlState: ResourceControlState.Idle,
  isSystem: false,
};

const systemContainer: TestContainer = {
  ...normalContainer,
  name: 'citadel',
  isSystem: true,
};

describe('system containers', () => {
  it('renders an accessible role-aware System badge', () => {
    render(<SystemContainerBadge role={ContainerSystemRole.Core} />);

    expect(screen.getByLabelText('Citadel Core')).toHaveAttribute('title', 'Citadel Core');
  });

  it('describes regular and Edge Agent system containers', () => {
    const { rerender } = render(<SystemContainerBadge role={ContainerSystemRole.Agent} />);

    expect(screen.getByLabelText('Citadel Agent')).toHaveAttribute('title', 'Citadel Agent');

    rerender(<SystemContainerBadge role={ContainerSystemRole.EdgeAgent} />);

    expect(screen.getByLabelText('Citadel Edge Agent')).toHaveAttribute('title', 'Citadel Edge Agent');
  });

  it('does not classify System containers as unmanaged', () => {
    expect(isUnmanagedContainer({ isSystem: true, deploymentId: null, stackId: null })).toBe(false);
    expect(isUnmanagedContainer({ isSystem: false, deploymentId: null, stackId: null })).toBe(true);
  });

  it('does not classify Swarm task containers as unmanaged', () => {
    expect(isUnmanagedContainer({ isSystem: false, isSwarmTask: true, deploymentId: null, stackId: null })).toBe(false);
  });

  it('does not classify containers with orphaned Citadel ownership labels as unmanaged', () => {
    expect(
      isUnmanagedContainer({
        isSystem: false,
        hasCitadelOwnershipLabels: true,
        deploymentId: null,
        stackId: null,
      }),
    ).toBe(false);
  });

  it('blocks lifecycle actions and mixed bulk selections containing a System container', () => {
    const actions = createContainerActions<TestContainer>(() => []);

    expect(actions.startAction.canExecute?.(systemContainer)).toBe(false);
    expect(actions.stopAction.canExecute?.(systemContainer)).toBe(false);
    expect(actions.pauseAction.primary.canExecute?.(systemContainer)).toBe(false);
    expect(actions.pauseAction.secondary.canExecute?.(systemContainer)).toBe(false);
    expect(actions.restartAction.canExecute?.(systemContainer)).toBe(false);
    expect(actions.restartAction.canExecute?.([normalContainer, systemContainer])).toBe(false);
    expect(containsSystemContainer([normalContainer, systemContainer])).toBe(true);
  });

  it('blocks direct lifecycle actions for Swarm task containers', () => {
    const actions = createContainerActions<TestContainer>(() => []);
    const taskContainer = { ...normalContainer, isSwarmTask: true };

    expect(actions.startAction.canExecute?.({ ...taskContainer, state: ContainerStateStatus.Exited })).toBe(false);
    expect(actions.stopAction.canExecute?.(taskContainer)).toBe(false);
    expect(actions.pauseAction.primary.canExecute?.(taskContainer)).toBe(false);
    expect(actions.restartAction.canExecute?.(taskContainer)).toBe(false);
  });
});

describe('unmanaged container import actions', () => {
  const standalone = {
    id: 'container-id',
    platformId: 'platform-id',
    name: 'standalone',
    containerId: 'docker-id',
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    isSystem: false,
    isSwarmTask: false,
    deploymentId: null,
    stackId: null,
    stack: null,
  } as ContainerView;

  const composeContainer = {
    ...standalone,
    name: 'compose-service',
    stack: 'compose-project',
  } as ContainerView;

  const composeSibling = {
    ...composeContainer,
    id: 'container-id-2',
    containerId: 'docker-id-2',
    name: 'compose-service-2',
  } as ContainerView;

  const composeRoot = {
    id: 'stack:compose-project',
    platformId: 'platform-id',
    name: 'compose-project',
    containerId: 'stack:compose-project',
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    stackId: null,
    stack: 'compose-project',
    lastStats: null,
    ports: {},
    deploymentId: null,
    isSystem: false,
    isSwarmTask: false,
    systemRole: null,
    imageView: null,
    displayStatus: StackReleaseStatus.Healthy,
    containers: [composeContainer, composeSibling],
    isStackGroup: true,
  } as ContainerStackGroupResource;

  it.each(['nginx', '/nginx', 'n', '/n'])('preserves the container name in the table: %s', (name) => {
    renderActions(<ContainersTable items={[{ ...standalone, name }]} isLoading={false} actions={{}} />);

    expect(screen.getByRole('link', { name: name.replace(/^\//, '') })).toHaveAttribute('title', name);
  });

  it.each(['nginx', '/nginx', 'n', '/n'])('preserves the shared container cell name: %s', (name) => {
    renderActions(
      <DockerContainerCell name={name} id="docker-id" state={ContainerStateStatus.Running} platformId="platform-id" />,
    );

    expect(screen.getByRole('link', { name: name.replace(/^\//, '') })).toHaveAttribute('title', name);
  });

  it('shows only adoption for a standalone unmanaged container', () => {
    expect(isAdoptableContainer(standalone)).toBe(true);
    expect(isContainerAdoptionAvailable(standalone, PlatformType.Docker)).toBe(true);
    expect(isContainerAdoptionAvailable(standalone, PlatformType.DockerSwarm)).toBe(false);
    expect(isImportableStack(standalone)).toBe(false);
  });

  it('shows the unmanaged marker only where direct container adoption is available', () => {
    const swarmView = renderActions(
      <ContainersTable items={[standalone]} isLoading={false} actions={{}} />,
      PlatformType.DockerSwarm,
    );

    expect(screen.queryByLabelText('Unmanaged Container')).not.toBeInTheDocument();

    swarmView.unmount();
    renderActions(<ContainersTable items={[standalone]} isLoading={false} actions={{}} />);

    expect(screen.getByLabelText('Unmanaged Container')).toBeInTheDocument();
  });

  it('does not show an ownership marker on an individual Swarm task', () => {
    const swarmTask = {
      ...standalone,
      name: 'redis.1.dwqiy5esa0jswywy',
      isSwarmTask: true,
    } as ContainerView;

    renderActions(<ContainersTable items={[swarmTask]} isLoading={false} actions={{}} />, PlatformType.DockerSwarm);

    expect(screen.queryByLabelText('Swarm Task')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Unmanaged Container')).not.toBeInTheDocument();
  });

  it('shows only stack import for an unmanaged Compose container', () => {
    expect(isAdoptableContainer(composeContainer)).toBe(false);
    expect(isImportableStack(composeContainer)).toBe(true);
  });

  it('allows stack import recovery for an unassigned Compose container with Citadel ownership labels', () => {
    const orphanedOwnedContainer = {
      ...composeContainer,
      hasCitadelOwnershipLabels: true,
    } as ContainerView;
    const orphanedRoot = {
      ...composeRoot,
      containers: [orphanedOwnedContainer, { ...composeSibling, hasCitadelOwnershipLabels: true }],
    } as ContainerStackGroupResource;

    expect(isAdoptableContainer(orphanedOwnedContainer)).toBe(false);
    expect(isImportableStack(orphanedOwnedContainer)).toBe(true);
    expect(isImportableStack(orphanedRoot)).toBe(true);
  });

  it('shows the applicable action in the single-selection action bar', () => {
    const { rerender } = renderActions(
      <>
        <ContainerGroupActions.adopt resources={[standalone]} />
        <ContainerGroupActions.importStack resources={[standalone]} />
      </>,
    );

    expect(screen.getByRole('button', { name: 'Adopt Container' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Import Stack' })).not.toBeInTheDocument();

    rerender(
      <ContainerActionsContext>
        <>
          <ContainerGroupActions.adopt resources={[composeContainer]} />
          <ContainerGroupActions.importStack resources={[composeContainer]} />
        </>
      </ContainerActionsContext>,
    );

    expect(screen.queryByRole('button', { name: 'Adopt Container' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Import Stack' })).toBeInTheDocument();
  });

  it('shows stack import for a selected root or several children of the same Compose project', () => {
    expect(isImportableStackSelection([composeRoot])).toBe(true);
    expect(isImportableStackSelection([composeContainer, composeSibling])).toBe(true);

    const { rerender } = renderActions(<ContainerGroupActions.importStack resources={[composeRoot]} />);

    expect(screen.getByRole('button', { name: 'Import Stack' })).toBeInTheDocument();

    rerender(
      <ContainerActionsContext>
        <ContainerGroupActions.importStack resources={[composeContainer, composeSibling]} />
      </ContainerActionsContext>,
    );

    expect(screen.getByRole('button', { name: 'Import Stack' })).toBeInTheDocument();
  });

  it('hides stack import for containers from different Compose projects', () => {
    const otherProject = { ...composeSibling, stack: 'other-project' } as ContainerView;

    expect(isImportableStackSelection([composeContainer, otherProject])).toBe(false);

    renderActions(<ContainerGroupActions.importStack resources={[composeContainer, otherProject]} />);

    expect(screen.queryByRole('button', { name: 'Import Stack' })).not.toBeInTheDocument();
  });

  it('hides container adoption but offers Compose-to-Swarm import on a Swarm platform', () => {
    renderActions(
      <>
        <ContainerGroupActions.adopt resources={[standalone]} />
        <ContainerGroupActions.importStack resources={[composeContainer]} />
        <DropdownMenu open>
          <DropdownMenuTrigger>Actions</DropdownMenuTrigger>
          <DropdownMenuContent forceMount>
            <ContainerDropdownActions.adopt resource={standalone} />
            <ContainerDropdownActions.importStack resource={composeContainer} />
          </DropdownMenuContent>
        </DropdownMenu>
      </>,
      PlatformType.DockerSwarm,
    );

    expect(screen.queryByRole('button', { name: 'Adopt Container' })).not.toBeInTheDocument();
    expect(screen.getAllByText('Import Stack')).toHaveLength(2);
    expect(screen.getByRole('menuitem', { name: 'Import Stack' })).toBeInTheDocument();
  });

  it('offers namespace import from a Swarm Stack root but not from an individual task', () => {
    const swarmTask = {
      ...composeContainer,
      name: 'redis-test_web.1.task-id',
      stack: 'redis-test',
      isSwarmTask: true,
    } as ContainerView;
    const swarmRoot = {
      ...composeRoot,
      id: 'stack:redis-test',
      name: 'redis-test',
      stack: 'redis-test',
      isSwarmTask: true,
      containers: [swarmTask],
    } as ContainerStackGroupResource;

    renderActions(
      <>
        <ContainerGroupActions.adopt resources={[swarmRoot]} />
        <ContainerGroupActions.importStack resources={[swarmRoot]} />
        <ContainerDropdownActions.adopt resource={swarmTask} />
        <ContainerDropdownActions.importStack resource={swarmTask} />
      </>,
      PlatformType.DockerSwarm,
    );

    expect(screen.queryByRole('button', { name: 'Adopt Container' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Import Stack' })).toBeInTheDocument();
  });
});
