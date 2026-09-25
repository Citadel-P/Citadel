import { ContainerView } from '@/api/generated/api.types';

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
