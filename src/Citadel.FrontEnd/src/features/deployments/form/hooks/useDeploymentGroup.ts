import { useCallback } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useQueryClient } from '@tanstack/react-query';
import { DeploymentView } from '@/api/generated/api.types';
import { ResourceResponse } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useDeploymentGroup = (deploymentId: string) => {
  const { data, isLoading, error, refetch, isFetching } = useRead('getDeployment', { deploymentId });
  const queryClient = useQueryClient();

  const handleDeploymentInfoUpdated = useCallback(
    (deployment: DeploymentView) => {
      if (deployment.id !== deploymentId) return;

      queryClient.setQueryData<ResourceResponse<'getDeployment'>>(['getDeployment', { deploymentId }], (current) =>
        current?.data
          ? {
              ...current,
              data: mergeDeploymentInfo(current.data, deployment),
            }
          : current,
      );
    },
    [deploymentId, queryClient],
  );

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useRealtimeGroup({
    groupName: 'deployment:' + deploymentId,
    skip: !deploymentId,
    setupEventListeners,
    removeEventListeners,
  });

  return { error, refetch, isFetching, deployment: data?.data, isLoading };
};

export const mergeDeploymentInfo = (current: DeploymentView, update: DeploymentView): DeploymentView => {
  const spec = update.spec
    ? {
        ...update.spec,
        image: normalizeMessagePackUnion(update.spec.image) as DeploymentView['spec']['image'],
      }
    : current.spec;
  const info = normalizeMessagePackUnion(update.latestActivityView?.info) as NonNullable<
    DeploymentView['latestActivityView']
  >['info'];

  return {
    ...update,
    capabilities: update.capabilities ?? current.capabilities,
    spec,
    latestActivityView: update.latestActivityView ? { ...update.latestActivityView, info } : null,
  };
};

const normalizeMessagePackUnion = (value: unknown): unknown => {
  if (
    !Array.isArray(value) ||
    typeof value[0] !== 'string' ||
    value[1] === null ||
    typeof value[1] !== 'object' ||
    Array.isArray(value[1])
  ) {
    return value;
  }

  return { ...value[1], $type: value[0] };
};
