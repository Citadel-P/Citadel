import { PlatformType, StackView } from '@/api/generated/api.types';
import { pauseAction, startAction, stopAction, syncAction } from './actions';

const stack = (platformType: PlatformType) =>
  ({
    name: 'demo',
    platformType,
  }) as StackView;

describe('Stack action visibility', () => {
  it.each([
    ['Start', startAction],
    ['Stop', stopAction],
    ['Pause or resume', pauseAction],
    ['Reconcile drift', syncAction],
  ])('hides the %s action for Swarm Stacks', (_, action) => {
    expect(action.isVisible?.(stack(PlatformType.DockerSwarm))).toBe(false);
  });

  it.each([
    ['Start', startAction],
    ['Stop', stopAction],
    ['Pause or resume', pauseAction],
    ['Reconcile drift', syncAction],
  ])('keeps the %s action for Standalone Stacks', (_, action) => {
    expect(action.isVisible?.(stack(PlatformType.Docker))).toBe(true);
  });
});
