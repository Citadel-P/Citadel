import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router';
import { ContainerStateStatus, PlatformType, PlatformView, ResourceControlState } from '@/api/generated/api.types';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerInfoActions, getContainerManagementAction } from './actions';
import { AppContext } from '@/lib/context/app-context';
import type { ReactNode } from 'react';

const standaloneContainer: ContainerDetailsView = {
  capabilities: null,
  containerStat: null,
  created: 0,
  ports: {},
  resourceId: '019f0000-0000-7000-8000-000000000001',
  platformId: '019f0000-0000-7000-8000-000000000002',
  id: 'ea5f935b4706',
  name: 'nginx',
  image: 'nginx:latest',
  imageId: 'nginx-image-id',
  state: ContainerStateStatus.Running,
  controlState: ResourceControlState.Idle,
  isSystem: false,
  hasCitadelOwnershipLabels: false,
  isSwarmTask: false,
  dockerNodeId: null,
  systemRole: null,
  stack: null,
  deploymentId: null,
  stackId: null,
};

const LocationProbe = () => {
  const location = useLocation();
  return <output data-testid="location">{`${location.pathname}${location.search}`}</output>;
};

const ContainerActionContext = ({
  children,
  platformType = PlatformType.Docker,
}: {
  children: ReactNode;
  platformType?: PlatformType;
}) => (
  <AppContext.Provider
    value={{
      isLoading: false,
      currentPlatform: { id: standaloneContainer.platformId, type: platformType } as PlatformView,
      platforms: [],
      applicationInfo: undefined,
      unresolvedAlertCount: 0,
      liveAlertEvents: {},
      receivedAlertEventIds: [],
    }}>
    <MemoryRouter>{children}</MemoryRouter>
  </AppContext.Provider>
);

describe('container detail management action', () => {
  it('opens adoption for an unassigned container with old ownership labels', async () => {
    const user = userEvent.setup();

    render(
      <ContainerActionContext>
        <ContainerInfoActions.adopt resource={{ ...standaloneContainer, hasCitadelOwnershipLabels: true }} />
        <LocationProbe />
      </ContainerActionContext>,
    );

    await user.click(screen.getByRole('button', { name: 'Adopt Container' }));

    expect(screen.getByTestId('location')).toHaveTextContent(
      '/deployments/add?adoptFrom=019f0000-0000-7000-8000-000000000001',
    );
  });

  it('selects stack import only for an unmanaged Compose container', () => {
    const composeContainer = { ...standaloneContainer, stack: 'demo-project' };

    expect(getContainerManagementAction(standaloneContainer, PlatformType.Docker)).toBe(ContainerInfoActions.adopt);
    expect(getContainerManagementAction(composeContainer, PlatformType.Docker)).toBe(ContainerInfoActions.importStack);
    expect(
      getContainerManagementAction({ ...standaloneContainer, deploymentId: 'deployment-id' }, PlatformType.Docker),
    ).toBeUndefined();
    expect(getContainerManagementAction(standaloneContainer, PlatformType.DockerSwarm)).toBeUndefined();
    expect(getContainerManagementAction(composeContainer, PlatformType.DockerSwarm)).toBe(
      ContainerInfoActions.importStack,
    );
  });

  it('opens stack import with the container platform and Compose project', async () => {
    const user = userEvent.setup();
    const composeContainer = { ...standaloneContainer, stack: 'demo project' };

    render(
      <ContainerActionContext platformType={PlatformType.DockerSwarm}>
        <ContainerInfoActions.importStack resource={composeContainer} />
        <LocationProbe />
      </ContainerActionContext>,
    );

    await user.click(screen.getByRole('button', { name: 'Import Stack' }));

    expect(screen.getByTestId('location')).toHaveTextContent(
      '/stacks/add?importPlatform=019f0000-0000-7000-8000-000000000002&importProject=demo+project&importKind=ComposeProject',
    );
  });

  it('disables direct adoption actions on a Swarm platform', () => {
    render(
      <ContainerActionContext platformType={PlatformType.DockerSwarm}>
        <ContainerInfoActions.adopt resource={standaloneContainer} />
      </ContainerActionContext>,
    );

    expect(screen.getByRole('button', { name: 'Adopt Container' })).toBeDisabled();
  });
});
