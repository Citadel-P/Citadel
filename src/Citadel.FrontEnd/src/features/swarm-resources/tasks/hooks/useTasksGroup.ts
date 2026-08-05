import type { PlatformCapabilities, SwarmTaskView } from '@/api/generated/api.types';
import type { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { getTaskName } from '@/lib/utils';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';

export type SwarmTaskInfoView = SwarmTaskView & {
  platformId: string;
  capabilities?: PlatformCapabilities;
};

const TASK_LIMIT = 200;
const selectTasks = (inventory: SwarmInventoryUpdate) => inventory.tasks.items.slice(0, TASK_LIMIT);

export const useTasksGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId, query: { limit: TASK_LIMIT } }), [platformId]);
  const query = useRead('listSwarmTasks', args);
  const items = useLiveSwarmItems(platformId, 'listSwarmTasks', args, query, selectTasks);
  return { items, isLoading: query.isLoading };
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
              task.capabilities ?? (currentPlatform?.id === platformId ? currentPlatform.capabilities : undefined),
          }
        : undefined,
    [currentPlatform, platformId, task],
  );
  return { resource, isLoading: query.isLoading, error: query.error };
};
