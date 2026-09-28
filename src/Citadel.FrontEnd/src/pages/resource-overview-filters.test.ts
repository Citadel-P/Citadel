import { ContainerComponents } from '@/features/docker-resources/containers';
import { SwarmServiceComponents } from '@/features/swarm-services';
import { StackComponents } from '@/features/stacks';
import { DeploymentComponents } from '@/features/deployments';
import { PlatformComponents } from '@/features/platforms';

describe('Resource overview categories', () => {
  it('puts stale running containers in other states until their projection is current', () => {
    const filters = ContainerComponents.overview!.filters;
    const stale = { state: 'Running', projectionStaleSince: '2026-09-27T00:00:00Z' } as never;
    expect(filters.find((filter) => filter.id === 'running')!.matches!(stale)).toBe(false);
    expect(filters.find((filter) => filter.id === 'other')!.matches!(stale)).toBe(true);
  });

  it('does not classify global services or unknown replica counts as scaled to zero', () => {
    const matches = SwarmServiceComponents.overview!.filters.find((filter) => filter.id === 'zero')!.matches!;
    expect(matches({ spec: { schedulingMode: 'Global', replicas: 0 } } as never)).toBe(false);
    expect(matches({ spec: { schedulingMode: 'Replicated', replicas: null } } as never)).toBe(false);
    expect(matches({ spec: { schedulingMode: 'Replicated', replicas: '0' } } as never)).toBe(true);
  });

  it('includes rollbacks among updating services', () => {
    const matches = SwarmServiceComponents.overview!.filters.find((filter) => filter.id === 'updating')!.matches!;
    expect(matches({ health: 'Healthy', updateState: 'rollback_started' } as never)).toBe(true);
    expect(matches({ health: 'Healthy', updateState: 'completed' } as never)).toBe(false);
  });

  it('does not count intentionally stopped workloads as needing attention', () => {
    for (const config of [StackComponents, DeploymentComponents]) {
      const matches = config.overview!.filters.find((filter) => filter.id === 'attention')!.matches!;
      expect(matches({ status: 'Stopped' } as never)).toBe(false);
      expect(matches({ status: 'Degraded' } as never)).toBe(true);
      expect(matches({ status: 'Failed' } as never)).toBe(true);
    }
  });

  it('filters offline platforms independently of their workload counts', () => {
    const matches = PlatformComponents.overview!.filters.find((filter) => filter.id === 'offline')!.matches!;
    expect(matches({ status: 'Offline', deploymentCount: 4 } as never)).toBe(true);
    expect(matches({ status: 'Online', deploymentCount: 0 } as never)).toBe(false);
  });
});
