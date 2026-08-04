import { PlatformType } from '@/api/generated/api.types';
import { PlatformMenu } from './menu-items';

describe('PlatformMenu', () => {
  it('uses the Docker resource menu for standalone platforms', () => {
    const menu = PlatformMenu({ id: 'docker-1', name: 'Docker', type: PlatformType.Docker });

    expect(menu.children?.map((item) => item.label)).toEqual(['Containers', 'Images', 'Networks', 'Volumes']);
  });

  it('uses a flat cluster and manager resource menu for Swarm platforms', () => {
    const menu = PlatformMenu({ id: 'swarm-1', name: 'Swarm', type: PlatformType.DockerSwarm });

    expect(menu.children?.map((item) => item.label)).toEqual([
      'Nodes',
      'Services',
      'Tasks',
      'Secrets',
      'Configs',
      'Containers',
      'Volumes',
      'Networks',
      'Images',
    ]);
    const nodes = menu.children?.[0];
    expect(nodes?.route).toBe('/platforms/swarm-1/swarm/nodes');
    expect(nodes?.disabled).not.toBe(true);
    expect(menu.children?.every((item) => item.route && !item.disabled && !item.children)).toBe(true);
    expect(menu.children?.find((item) => item.label === 'Networks')?.route).toBe('/platforms/swarm-1/swarm/networks');
  });
});
