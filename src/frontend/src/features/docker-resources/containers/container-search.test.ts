import { describe, expect, it } from 'vitest';
import type { ContainerView } from '@/api/generated/api.types';
import { filterContainers } from './container-search';

const container = {
  name: 'api-1',
  containerId: 'abcdef1234567890',
  dockerImageId: 'sha256:fedcba0987654321',
  imageView: { name: 'registry.example/app', tags: ['registry.example/app:release-42'] },
  ownerNames: ['Checkout Deployment', 'Production Stack', 'Payments Service'],
  stack: 'hidden-stack-label',
  deploymentView: { name: 'hidden-deployment' },
} as ContainerView;

describe('container search', () => {
  it.each([
    'API-1',
    'abcdef123456',
    'abcdef1234567890',
    'fedcba0987654321',
    'registry.example/app',
    'RELEASE-42',
    'checkout',
    'production stack',
    'payments service',
  ])('matches %s', (term) => {
    expect(filterContainers([container], ` ${term} `)).toEqual([container]);
  });

  it('does not search owner names outside the authorized projection', () => {
    expect(filterContainers([container], 'hidden')).toEqual([]);
    expect(filterContainers([{ ...container, ownerNames: [] }], 'checkout')).toEqual([]);
  });

  it('handles absent image and owner metadata without changing order', () => {
    const bare = { name: 'api-2', containerId: '1234' } as ContainerView;
    const items = [bare, container];
    expect(filterContainers(items, '  ')).toBe(items);
    expect(filterContainers(items, 'api')).toEqual(items);
    expect(filterContainers(items, 'missing')).toEqual([]);
  });
});
