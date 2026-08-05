import { PlatformCapabilities, SwarmNodeView, SwarmTaskView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { useContext, useMemo } from 'react';
import { useLiveSwarmItems, useLiveSwarmResource } from '../../hooks/useSwarmResourceGroup';
import { useTasksGroup } from '../../tasks/hooks/useTasksGroup';

export type SwarmNodeListView = SwarmNodeView & { tasks: SwarmTaskView[] };

export type SwarmNodeInfoView = SwarmNodeView & {
  name: string;
  platformId: string;
  capabilities?: PlatformCapabilities;
  tasks: SwarmTaskView[];
};

const selectNodes = (inventory: SwarmInventoryUpdate) => inventory.nodes.items;

export const useNodesGroup = (platformId: string) => {
  const args = useMemo(() => ({ platformId }), [platformId]);
  const query = useRead('listSwarmNodes', args);
  const nodes = useLiveSwarmItems(platformId, 'listSwarmNodes', args, query, selectNodes);
  const taskGroup = useTasksGroup(platformId);
  const tasksByNode = useMemo(() => groupTasksByNode(taskGroup.items), [taskGroup.items]);
  const items = useMemo<SwarmNodeListView[]>(
    () => nodes.map((node) => ({ ...node, tasks: tasksByNode.get(node.id) ?? [] })),
    [nodes, tasksByNode],
  );
  return { items, isLoading: query.isLoading || taskGroup.isLoading };
};

export const useNodeInfoGroup = (platformId: string, nodeId: string) => {
  const currentPlatform = useContext(AppContext)?.currentPlatform;
  const args = useMemo(() => ({ platformId, nodeId }), [nodeId, platformId]);
  const query = useRead('getSwarmNode', args);
  const taskGroup = useTasksGroup(platformId);
  const node = useLiveSwarmResource(
    platformId,
    nodeId,
    'getSwarmNode',
    args,
    `/platforms/${platformId}/nodes`,
    query,
    selectNodes,
  );
  const resource = useMemo<SwarmNodeInfoView | undefined>(
    () =>
      node
        ? {
            ...node,
            name: node.hostname || node.id,
            platformId,
            capabilities:
              node.capabilities ?? (currentPlatform?.id === platformId ? currentPlatform.capabilities : undefined),
            tasks: taskGroup.items.filter((task) => task.nodeId === node.id),
          }
        : undefined,
    [currentPlatform, node, platformId, taskGroup.items],
  );
  return {
    resource,
    isLoading: query.isLoading || taskGroup.isLoading,
    error: query.error,
  };
};

const groupTasksByNode = (tasks: SwarmTaskView[]) => {
  const result = new Map<string, SwarmTaskView[]>();
  for (const task of tasks) {
    const nodeTasks = result.get(task.nodeId);
    if (nodeTasks) nodeTasks.push(task);
    else result.set(task.nodeId, [task]);
  }
  return result;
};
