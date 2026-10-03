import { ContainerStateStatus, type ContainerView } from '@/api/generated/api.types';
import { isContainerStackGroup, type ContainerActionResource } from './actions';

export const isVisibleContainer = (container: Pick<ContainerView, 'isSwarmTask' | 'state'>) =>
  !container.isSwarmTask ||
  (container.state !== ContainerStateStatus.Exited &&
    container.state !== ContainerStateStatus.Dead &&
    container.state !== ContainerStateStatus.Removing);

export const filterVisibleContainers = <T extends Pick<ContainerView, 'isSwarmTask' | 'state'>>(containers: T[]) =>
  containers.filter(isVisibleContainer);

export const normalizeContainerSelection = (selectedRows: ContainerActionResource[]) => {
  const groupedContainerIds = new Set(
    selectedRows
      .filter(isContainerStackGroup)
      .flatMap((group) => group.containers.map((container) => container.id)),
  );
  const seen = new Set<string>();

  return selectedRows.filter((resource) => {
    if (!isContainerStackGroup(resource) && groupedContainerIds.has(resource.id)) return false;

    const key = resource.id;
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
