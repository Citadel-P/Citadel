import { ContainerView } from '@/api/generated/api.types';

export const reconcileContainerOrder = (
  previousContainers: ContainerView[] | undefined,
  incomingContainers: ContainerView[],
): ContainerView[] => {
  if (!previousContainers?.length || incomingContainers.length < 2) {
    return incomingContainers;
  }

  const previousIds = new Set(previousContainers.map((container) => container.containerId));
  const incomingById = new Map(incomingContainers.map((container) => [container.containerId, container]));
  const reconciled: ContainerView[] = [];

  for (const container of incomingContainers) {
    if (!previousIds.has(container.containerId)) {
      reconciled.push(container);
    }
  }

  for (const container of previousContainers) {
    const updatedContainer = incomingById.get(container.containerId);
    if (updatedContainer) {
      reconciled.push(updatedContainer);
    }
  }

  return reconciled;
};
