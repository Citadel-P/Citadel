import { isContainerStackGroup, type ContainerActionResource } from './actions';

export const normalizeContainerSelection = (selectedRows: ContainerActionResource[]) => {
  const groupedContainerIds = new Set(
    selectedRows
      .filter(isContainerStackGroup)
      .flatMap((group) => group.containers.map((container) => container.containerId)),
  );
  const seen = new Set<string>();

  return selectedRows.filter((resource) => {
    if (!isContainerStackGroup(resource) && groupedContainerIds.has(resource.containerId)) return false;

    const key = isContainerStackGroup(resource) ? resource.id : resource.containerId;
    if (seen.has(key)) return false;

    seen.add(key);
    return true;
  });
};

export const countSelectedContainers = (selectedResources: ContainerActionResource[]) =>
  selectedResources.reduce(
    (count, resource) => count + (isContainerStackGroup(resource) ? resource.containers.length : 1),
    0,
  );
