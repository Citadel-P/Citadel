import { PlatformType } from '@/api/generated/api.types';
import { PlatformMenu } from './menu-items';

describe('PlatformMenu', () => {
  it('uses the Docker resource menu for standalone platforms', () => {
    const menu = PlatformMenu({ id: 'docker-1', name: 'Docker', type: PlatformType.Docker });

    expect(menu.children?.map((item) => item.label)).toEqual(['Containers', 'Images', 'Networks', 'Volumes']);
  });

  it('uses cluster and manager groups for Swarm platforms', () => {
    const menu = PlatformMenu({ id: 'swarm-1', name: 'Swarm', type: PlatformType.DockerSwarm });

    expect(menu.children?.map((item) => item.label)).toEqual([
      'Overview',
      'Cluster',
      'Resources',
      'Orchestration',
      'Connected manager',
    ]);
    expect(menu.children?.find((item) => item.label === 'Cluster')?.children?.map((item) => item.label)).toEqual([
      'Nodes',
      'Services',
      'Tasks',
    ]);
    expect(
      menu.children?.find((item) => item.label === 'Connected manager')?.children?.map((item) => item.label),
    ).toEqual(['Containers', 'Images', 'Volumes']);
  });
});
