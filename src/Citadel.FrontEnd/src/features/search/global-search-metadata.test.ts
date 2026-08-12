import { GlobalSearchCategory, GlobalSearchResourceType } from '@/api/generated/api.types';
import { describe, expect, it } from 'vitest';
import { globalSearchCategories, globalSearchResourceMetadata } from './global-search-metadata';

describe('globalSearchResourceMetadata', () => {
  it('routes managed Swarm Service results to their list and detail pages', () => {
    const metadata = globalSearchResourceMetadata[GlobalSearchResourceType.SwarmService];

    expect(metadata.category).toBe(GlobalSearchCategory.SwarmServices);
    expect(metadata.listPath).toBe('/swarm-services');
    expect(metadata.getDetailsPath('service-1')).toBe('/swarm-services/edit/service-1');
    expect(globalSearchCategories).toContainEqual(
      expect.objectContaining({
        category: GlobalSearchCategory.SwarmServices,
        listPath: '/swarm-services',
      }),
    );
  });
});
