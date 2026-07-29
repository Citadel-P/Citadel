import { render, screen } from '@testing-library/react';
import { ContainerStateStatus, ContainerSystemRole, ResourceControlState } from '@/api/generated/api.types';
import { containsSystemContainer, createContainerActions } from './actions';
import { SystemContainerBadge } from './system-container-badge';
import { isUnmanagedContainer } from '@/lib/utils';

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
