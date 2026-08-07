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
  isImportableStack,
  isImportableStackSelection,
} from './actions';
import { SystemContainerBadge } from './system-container-badge';
import { isUnmanagedContainer } from '@/lib/utils';
import { AppContext } from '@/lib/context/app-context';
import type { ReactNode } from 'react';

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
    systemRole: null,
    imageView: null,
    displayStatus: StackReleaseStatus.Healthy,
    containers: [composeContainer, composeSibling],
    isStackGroup: true,
  } as ContainerStackGroupResource;

  it('shows only adoption for a standalone unmanaged container', () => {
    expect(isAdoptableContainer(standalone)).toBe(true);
    expect(isImportableStack(standalone)).toBe(false);
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

  it('hides standalone adoption and Compose import on a Swarm platform', () => {
    renderActions(
      <>
        <ContainerGroupActions.adopt resources={[standalone]} />
        <ContainerGroupActions.importStack resources={[composeContainer]} />
        <ContainerDropdownActions.adopt resource={standalone} />
        <ContainerDropdownActions.importStack resource={composeContainer} />
      </>,
      PlatformType.DockerSwarm,
    );

    expect(screen.queryByRole('button', { name: 'Adopt Container' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Import Stack' })).not.toBeInTheDocument();
  });
});
