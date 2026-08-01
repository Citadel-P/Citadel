import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router';
import { ContainerStateStatus, ResourceControlState } from '@/api/generated/api.types';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerInfoActions, getContainerManagementAction } from './actions';

const standaloneContainer: ContainerDetailsView = {
  resourceId: '019f0000-0000-7000-8000-000000000001',
  platformId: '019f0000-0000-7000-8000-000000000002',
  id: 'ea5f935b4706',
  name: 'nginx',
  image: 'nginx:latest',
  imageId: 'nginx-image-id',
  state: ContainerStateStatus.Running,
  controlState: ResourceControlState.Idle,
  isSystem: false,
  systemRole: null,
  stack: null,
  deploymentId: null,
  stackId: null,
};

const LocationProbe = () => {
  const location = useLocation();
  return <output data-testid="location">{`${location.pathname}${location.search}`}</output>;
};

describe('container detail management action', () => {
  it('opens deployment adoption with the persisted container id', async () => {
    const user = userEvent.setup();

    render(
      <MemoryRouter>
        <ContainerInfoActions.adopt resource={standaloneContainer} />
        <LocationProbe />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('button', { name: 'Adopt Container' }));

    expect(screen.getByTestId('location')).toHaveTextContent(
      '/deployments/add?adoptFrom=019f0000-0000-7000-8000-000000000001',
    );
  });

  it('selects stack import only for an unmanaged Compose container', () => {
    const composeContainer = { ...standaloneContainer, stack: 'demo-project' };

    expect(getContainerManagementAction(standaloneContainer)).toBe(ContainerInfoActions.adopt);
    expect(getContainerManagementAction(composeContainer)).toBe(ContainerInfoActions.importStack);
    expect(getContainerManagementAction({ ...standaloneContainer, deploymentId: 'deployment-id' })).toBeUndefined();
  });

  it('opens stack import with the container platform and Compose project', async () => {
    const user = userEvent.setup();
    const composeContainer = { ...standaloneContainer, stack: 'demo project' };

    render(
      <MemoryRouter>
        <ContainerInfoActions.importStack resource={composeContainer} />
        <LocationProbe />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('button', { name: 'Import Stack' }));

    expect(screen.getByTestId('location')).toHaveTextContent(
      '/stacks/add?importPlatform=019f0000-0000-7000-8000-000000000002&importProject=demo+project',
    );
  });
});
