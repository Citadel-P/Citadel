import { ContainerView } from '@/api/generated/api.types';

export interface ContainerStatePatch {
  id: string;
  containerId: string;
  state?: ContainerView['state'];
  controlState?: ContainerView['controlState'];
  updated?: number;
  dockerNodeId?: string | null;
}

export const applyContainerStatePatches = (
  previous: ContainerView[],
  patches: ContainerStatePatch[],
): ContainerView[] => {
  if (!patches.length) return previous;
  const byId = new Map(patches.map((patch) => [patch.id, patch]));
  let changed = false;
  const updated = previous.map((container) => {
    const patch = byId.get(container.id);
    if (!patch) return container;
    const next = {
      ...container,
      ...(patch.state !== undefined ? { state: patch.state } : {}),
      ...(patch.controlState !== undefined ? { controlState: patch.controlState } : {}),
      ...(patch.updated !== undefined ? { updated: patch.updated } : {}),
    };
    if (
      next.state !== container.state ||
      next.controlState !== container.controlState ||
      next.updated !== container.updated
    ) {
      changed = true;
    }
    return next;
  });
  return changed ? updated : previous;
};

export const applyContainerChange = (
  previous: ContainerView[],
  dockerId: string,
  incoming: ContainerView[],
): ContainerView[] =>
  reconcileContainerOrder(previous, [
    ...previous.filter((container) => container.containerId !== dockerId),
    ...incoming,
  ]);

export const reconcileContainerOrder = (
  previousContainers: ContainerView[] | undefined,
  incomingContainers: ContainerView[],
): ContainerView[] => {
  if (!previousContainers?.length || incomingContainers.length < 2) {
    return incomingContainers;
  }

  const previousIds = new Set(previousContainers.map((container) => container.id));
  const incomingById = new Map(incomingContainers.map((container) => [container.id, container]));
  const reconciled: ContainerView[] = [];

  for (const container of incomingContainers) {
    if (!previousIds.has(container.id)) {
      reconciled.push(container);
    }
  }

  for (const container of previousContainers) {
    const updatedContainer = incomingById.get(container.id);
    if (updatedContainer) {
      reconciled.push(updatedContainer);
    }
  }

  return reconciled;
};
