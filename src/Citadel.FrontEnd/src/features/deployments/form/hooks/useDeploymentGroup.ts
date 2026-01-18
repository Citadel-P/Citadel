import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { DeploymentView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useDeploymentGroup = (deploymentId: string) => {
  const { data, isLoading } = useRead('getDeployment', { deploymentId });
  const [deployment, setDeployment] = useState<DeploymentView | undefined>();

  useEffect(() => {
    if (!data) return;
    setDeployment(data.data);
  }, [data]);

  const handleDeploymentInfoUpdated = useCallback((deployment: DeploymentView) => {
    setDeployment((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        name: deployment.name,
        status: deployment.status,
        description: deployment.description,
      };
    });
  }, []);

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

  return { deployment, isLoading };
};
