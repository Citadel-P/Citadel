import { PlatformCapabilities, SwarmServiceView, SwarmTaskView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';
import { useTasksGroup } from '../../tasks/hooks/useTasksGroup';

export type SwarmServiceListView = SwarmServiceView & { tasks: SwarmTaskView[] };

export type SwarmServiceInfoView = SwarmServiceView & {
  platformId: string;
  capabilities?: PlatformCapabilities;
  tasks: SwarmTaskView[];
};

const selectServices = (inventory: SwarmInventoryUpdate) => inventory.services.items;

export const useServicesGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmServices', args);
  const services = useLiveSwarmItems(platformId, 'listSwarmServices', args, query, selectServices);
  const taskGroup = useTasksGroup(platformId);
  const tasksByService = useMemo(() => groupTasksByService(taskGroup.items), [taskGroup.items]);
  const items = useMemo<SwarmServiceListView[]>(
    () => services.map((service) => ({ ...service, tasks: tasksByService.get(service.id) ?? [] })),
    [services, tasksByService],
  );
  return { items, isLoading: query.isLoading || taskGroup.isLoading };
};

export const useServiceInfoGroup = (platformId: string, resourceId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmService', args);
  const taskGroup = useTasksGroup(platformId);
  const service = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmService',
    args,
    `/platforms/${platformId}/services`,
    query,
    selectServices,
  );
  const resource = useMemo<SwarmServiceInfoView | undefined>(
    () =>
      service
        ? {
            ...service,
            platformId,
            capabilities:
              service.capabilities ??
              (currentPlatform?.id === platformId ? currentPlatform.capabilities : undefined),
            tasks: taskGroup.items.filter((task) => task.serviceId === service.id),
          }
        : undefined,
    [currentPlatform, platformId, service, taskGroup.items],
  );
  return {
    resource,
    isLoading: query.isLoading || taskGroup.isLoading,
    error: query.error,
  };
};

const groupTasksByService = (tasks: SwarmTaskView[]) => {
  const result = new Map<string, SwarmTaskView[]>();
  for (const task of tasks) {
    const serviceTasks = result.get(task.serviceId);
    if (serviceTasks) serviceTasks.push(task);
    else result.set(task.serviceId, [task]);
  }
  return result;
};
