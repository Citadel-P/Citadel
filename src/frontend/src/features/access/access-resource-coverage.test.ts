import { ResourceType } from '@/api/generated/api.types';
import { OVERRIDE_RESOURCE_CONFIG, OVERRIDE_RESOURCE_TYPES } from './overrides/resource-overrides-field';
import { RESOURCE_ICONS, ROLE_PERMISSION_RESOURCES } from './roles';

describe('access resource metadata', () => {
  it('makes managed Swarm Services configurable in roles', () => {
    expect(ROLE_PERMISSION_RESOURCES).toContain(ResourceType.SwarmService);
    expect(RESOURCE_ICONS[ResourceType.SwarmService]).toBeDefined();
  });

  it('makes managed Swarm Services available for resource overrides', () => {
    expect(OVERRIDE_RESOURCE_TYPES).toContain(ResourceType.SwarmService);
    expect(OVERRIDE_RESOURCE_CONFIG[ResourceType.SwarmService]).toMatchObject({
      listQuery: 'listManagedSwarmServices',
    });

    expect(
      OVERRIDE_RESOURCE_CONFIG[ResourceType.SwarmService].readItems({
        swarmServices: [{ id: 'service-id', name: 'redis' }],
      }),
    ).toEqual([{ id: 'service-id', name: 'redis' }]);
    expect(OVERRIDE_RESOURCE_CONFIG[ResourceType.SwarmService].getEditPath('service-id')).toBe(
      '/swarm-services/edit/service-id',
    );
  });
});
