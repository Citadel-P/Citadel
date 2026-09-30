import type { PlatformCapabilities, SwarmServiceView, SwarmTaskView } from '@/api/generated/api.types';
import type { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { getTaskName } from '@/lib/utils';
import { useCallback, useContext, useMemo, useState } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

export type SwarmTaskInfoView = SwarmTaskView & {
  platformId: string;
  capabilities?: PlatformCapabilities | null;
};

const TASK_LIMIT = 200;
const selectTasks = (inventory: SwarmInventoryUpdate) => inventory.tasks.items.slice(0, TASK_LIMIT);

export const useTasksGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId, query: { limit: TASK_LIMIT } }), [platformId]);
  const query = useRead('listSwarmTasks', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmTasks', args, query, selectTasks);
  return {
    items,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};

export const useServiceTasksGroup = (platformId: string, serviceId: string | null) => {
  const args = useMemo(
    () => ({ platformId, query: { limit: TASK_LIMIT, serviceId: serviceId ?? undefined } }),
    [platformId, serviceId],
  );
  const query = useRead('listSwarmTasks', args);
  const [liveRuntime, setLiveRuntime] = useState<{ platformId: string; service: SwarmServiceView }>();
  const selectServiceTasks = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (!serviceId) return [];

      const tasks: SwarmTaskView[] = [];
      for (const task of inventory.tasks.items) {
        if (task.serviceId !== serviceId) continue;
        tasks.push(task);
        if (tasks.length === TASK_LIMIT) break;
      }
      return tasks;
    },
    [serviceId],
  );
  const updateRuntime = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (!serviceId) return;
      const service = inventory.services.items.find((item) => item.id === serviceId);
      setLiveRuntime(service ? { platformId: inventory.platformId, service } : undefined);
    },
    [serviceId],
  );
  const items = useLiveSwarmItems(platformId, 'listSwarmTasks', args, query, selectServiceTasks, updateRuntime);
  const runtime =
    liveRuntime?.platformId === platformId && liveRuntime.service.id === serviceId ? liveRuntime.service : undefined;

  return {
    items,
    runtime,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};

export const useTaskInfoGroup = (platformId: string, resourceId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, resourceId }), [platformId, resourceId]);
  const query = useRead('getSwarmTask', args);
  const task = useLiveSwarmResource(
    platformId,
    resourceId,
    'getSwarmTask',
    args,
    `/platforms/${platformId}/tasks`,
    query,
    (inventory) => inventory.tasks.items,
  );
  const resource = useMemo<SwarmTaskInfoView | undefined>(
    () =>
      task
        ? {
            ...task,
            name: getTaskName(task),
            platformId,
            capabilities:
              task.capabilities ?? (currentPlatform?.id === platformId ? (currentPlatform.capabilities ?? null) : null),
          }
        : undefined,
    [currentPlatform, platformId, task],
  );
  return {
    resource,
    refetch: query.refetch,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    error: query.error,
  };
};
