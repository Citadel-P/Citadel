import { useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useQueryClient } from '@tanstack/react-query';
import { DeploymentView } from '@/api/generated/api.types';
import { ResourceResponse } from '@/api/types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useDeploymentGroup = (deploymentId: string) => {
  const { data, isLoading } = useRead('getDeployment', { deploymentId });
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
    (hubConnection: HubConnection) => {
      hubConnection.on('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('DeploymentInfoUpdated', handleDeploymentInfoUpdated);
    },
    [handleDeploymentInfoUpdated],
  );

  useSignalRGroup({
    groupName: 'deployment:' + deploymentId,
    skip: !deploymentId,
    setupEventListeners,
    removeEventListeners,
  });

  return { deployment: data?.data, isLoading };
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
