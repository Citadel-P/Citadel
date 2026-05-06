import { useEffect, useState, useCallback, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { DeploymentView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useDeploymentsGroup = () => {
  const { data, isLoading } = useRead('listDeployments');
  const [deployments, setDeployments] = useState<DeploymentView[] | undefined>();
  const lastFetchedRef = useRef<DeploymentView[]>([]);

  useEffect(() => {
    if (!data) return;
    const newBase = data.data.deployments;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setDeployments(newBase);
    }
  }, [data]);

  const handleDeploymentInfoUpdated = useCallback((deployment: DeploymentView, action: string) => {
    setDeployments((prev) => {
      if (!prev) return prev;

      if (action === 'create') {
        return [...prev, deployment];
      }
      if (action === 'delete') {
        return prev.filter((d) => d.id !== deployment.id);
      }

      const index = prev.findIndex((d) => d.id === deployment.id);
      if (index !== -1) {
        const updated = [...prev];
        updated[index] = deployment;
        return updated;
      }
      return prev;
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
    groupName: 'deployments',
    setupEventListeners,
    removeEventListeners,
  });

  return { deployments, isLoading };
};
