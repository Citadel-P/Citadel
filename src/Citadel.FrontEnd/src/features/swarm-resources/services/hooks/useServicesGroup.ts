import {
  ManagedSwarmServiceView,
  PlatformCapabilitiesView,
  ResourceControlState,
  SwarmServiceCapabilities,
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
  | 'labels'
  | 'ownership'
  | 'ownershipDiagnostic'
  | 'dockerStackNamespace'
  | 'stackId'
  | 'isStale'
> & {
  capabilities?: null | PlatformCapabilitiesView | SwarmServiceCapabilities;
  tasks: SwarmTaskView[];
  managedServiceId?: string;
  managedControlState?: ResourceControlState;
  isManagedDraft?: boolean;
  canAdopt?: boolean;
};

export type SwarmServiceInfoView = SwarmServiceView & {
  platformId: string;
  capabilities?: PlatformCapabilitiesView | null;
  tasks: SwarmTaskView[];
  managedServiceId?: string;
  canAdopt?: boolean;
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
    const canAdopt = managedGroup.capabilities?.canWrite === true;
    const managedByDockerId = new Map(
      (managedGroup.services ?? [])
        .filter((service) => service.dockerServiceId)
        .map((service) => [service.dockerServiceId!, service]),
    );
    const runtimeIds = new Set(services.map((service) => service.id));
    const runtime = services.map((service) => {
      const managed = managedByDockerId.get(service.id);
      return {
        ...service,
        ownership: managed ? SwarmServiceOwnership.CitadelService : service.ownership,
        ownershipDiagnostic: managed ? null : service.ownershipDiagnostic,
        tasks: tasksByService.get(service.id) ?? [],
        managedServiceId: managed?.id,
        managedControlState: managed?.controlState,
        capabilities: managed?.capabilities ?? service.capabilities,
        canAdopt,
      };
    });
    const drafts = (managedGroup.services ?? [])
      .filter((service) => !service.dockerServiceId || !runtimeIds.has(service.dockerServiceId))
      .map(toDraftListView);
    return [...runtime, ...drafts];
  }, [managedGroup.capabilities?.canWrite, managedGroup.services, services, tasksByService]);
  return {
    items,
    capabilities: managedGroup.capabilities,
    error: query.error ?? taskGroup.error ?? managedGroup.error,
    refetch: () => Promise.all([query.refetch(), taskGroup.refetch(), managedGroup.refetch()]),
    isFetching: query.isFetching || taskGroup.isFetching || managedGroup.isFetching,
    isLoading: query.isLoading || taskGroup.isLoading || managedGroup.isLoading,
  };
};

const toDraftListView = (service: ManagedSwarmServiceView): SwarmServiceListView => ({
  id: service.dockerServiceId ?? service.id,
  name: service.name,
  mode: service.spec.schedulingMode ?? 'Replicated',
  image:
    'imageTag' in service.spec.image
      ? service.spec.image.imageTag
      : (service.spec.image.resolvedImageReference ?? 'Build output pending'),
  runningTaskCount: service.runningTaskCount ?? 0,
  desiredTaskCount: service.desiredTaskCount ?? service.spec.replicas ?? 0,
  updateState: service.updateState ?? 'Not deployed',
  labels: {},
  ownership: SwarmServiceOwnership.CitadelService,
  ownershipDiagnostic: null,
  dockerStackNamespace: null,
  stackId: null,
  isStale: false,
  capabilities: service.capabilities,
  tasks: [],
  managedServiceId: service.id,
  managedControlState: service.controlState,
  isManagedDraft: true,
  canAdopt: false,
});

export const useServiceInfoGroup = (platformId: string, resourceId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmService', args);
  const managedGroup = useSwarmServicesGroup(platformId);
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
  const resource = useMemo<SwarmServiceInfoView | undefined>(() => {
    if (!service) return undefined;

    const managed = managedGroup.services?.find((item) => item.dockerServiceId === service.id);
    return {
      ...service,
      ownership: managed ? SwarmServiceOwnership.CitadelService : service.ownership,
      ownershipDiagnostic: managed ? null : service.ownershipDiagnostic,
      platformId,
      capabilities:
        service.capabilities ?? (currentPlatform?.id === platformId ? (currentPlatform.capabilities ?? null) : null),
      tasks: taskGroup.items.filter((task) => task.serviceId === service.id),
      managedServiceId: managed?.id,
      canAdopt: managedGroup.capabilities?.canWrite === true,
    };
  }, [
    currentPlatform,
    managedGroup.capabilities?.canWrite,
    managedGroup.services,
    platformId,
    service,
    taskGroup.items,
  ]);
  return {
    resource,
    error: query.error ?? taskGroup.error ?? managedGroup.error,
    refetch: () => Promise.all([query.refetch(), taskGroup.refetch(), managedGroup.refetch()]),
    isFetching: query.isFetching || taskGroup.isFetching || managedGroup.isFetching,
    isLoading: query.isLoading || taskGroup.isLoading || managedGroup.isLoading,
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
