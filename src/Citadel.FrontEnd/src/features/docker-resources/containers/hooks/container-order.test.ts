import { describe, expect, it } from 'vitest';
import { ContainerView } from '@/api/generated/api.types';
import { reconcileContainerOrder } from './container-order';

const container = (containerId: string, state = 'running') =>
  ({
    id: `id-${containerId}`,
    containerId,
    state,
  }) as ContainerView;

describe('reconcileContainerOrder', () => {
  it('updates a reordered snapshot without moving existing containers', () => {
    const previous = [container('a'), container('b'), container('c')];
    const updatedA = container('a', 'paused');

    const result = reconcileContainerOrder(previous, [container('c'), updatedA, container('b')]);

    expect(result.map(({ containerId }) => containerId)).toEqual(['a', 'b', 'c']);
    expect(result[0]).toBe(updatedA);
  });

  it('keeps new containers first and removes containers missing from the snapshot', () => {
    const previous = [container('a'), container('b'), container('c')];
    const added = container('d');

    const result = reconcileContainerOrder(previous, [container('c'), added, container('a')]);

    expect(result.map(({ containerId }) => containerId)).toEqual(['d', 'a', 'c']);
    expect(result[0]).toBe(added);
  });

  it('uses the server order for the initial snapshot', () => {
    const incoming = [container('c'), container('a'), container('b')];

    expect(reconcileContainerOrder(undefined, incoming)).toBe(incoming);
  });
});
