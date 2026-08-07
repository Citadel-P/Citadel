import {
  ManagedSwarmServiceView,
  PlatformCapabilities,
  SwarmServiceOwnership,
  SwarmServiceView,
  SwarmTaskView,
} from '@/api/generated/api.types';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';
import { useTasksGroup } from '../../tasks/hooks/useTasksGroup';
import { useSwarmServicesGroup } from '@/features/swarm-services/hooks/useSwarmServicesGroup';

export type SwarmServiceListView = Pick<
  SwarmServiceView,
  | 'id'
  | 'name'
  | 'mode'
  | 'image'
  | 'runningTaskCount'
  | 'desiredTaskCount'
  | 'updateState'
  | 'ownership'
  | 'ownershipDiagnostic'
  | 'isStale'
> & {
  tasks: SwarmTaskView[];
  managedServiceId?: string;
  isManagedDraft?: boolean;
};

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
  const managedGroup = useSwarmServicesGroup(platformId);
  const taskGroup = useTasksGroup(platformId);
  const tasksByService = useMemo(() => groupTasksByService(taskGroup.items), [taskGroup.items]);
  const items = useMemo<SwarmServiceListView[]>(() => {
    const managedByDockerId = new Map(
      (managedGroup.services ?? [])
        .filter((service) => service.dockerServiceId)
        .map((service) => [service.dockerServiceId!, service]),
    );
    const runtimeIds = new Set(services.map((service) => service.id));
    const runtime = services.map((service) => ({
      ...service,
      tasks: tasksByService.get(service.id) ?? [],
      managedServiceId: managedByDockerId.get(service.id)?.id,
    }));
    const drafts = (managedGroup.services ?? [])
      .filter((service) => !service.dockerServiceId || !runtimeIds.has(service.dockerServiceId))
      .map(toDraftListView);
    return [...runtime, ...drafts];
  }, [managedGroup.services, services, tasksByService]);
  return {
    items,
    capabilities: managedGroup.capabilities,
    isLoading: query.isLoading || taskGroup.isLoading || managedGroup.isLoading,
  };
};

const toDraftListView = (service: ManagedSwarmServiceView): SwarmServiceListView => ({
  id: service.dockerServiceId ?? service.id,
  name: service.name,
  mode: service.spec.schedulingMode,
  image: 'imageTag' in service.spec.image
    ? service.spec.image.imageTag
    : service.spec.image.resolvedImageReference ?? 'Build output pending',
  runningTaskCount: service.runningTaskCount ?? 0,
  desiredTaskCount: service.desiredTaskCount ?? service.spec.replicas ?? 0,
  updateState: service.updateState ?? 'Not deployed',
  ownership: SwarmServiceOwnership.CitadelService,
  ownershipDiagnostic: null,
  isStale: false,
  tasks: [],
  managedServiceId: service.id,
  isManagedDraft: true,
});

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
