import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useAuthContext } from '@/features/auth/AuthContext';
import { useSignalRHub } from '@/hooks/useSignalRHub';

export const useContainerLogHub = (containerId?: string) => {
  const { accessToken } = useAuthContext();

  const [containerLog, setContainerLog] = useState<string>();
  const [containerLogs, setContainerLogs] = useState<string[]>([]);
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const handleContainerLog = useCallback((log: string) => {
    setContainerLog(log);
  }, []);

  const handleContainerLogsBatch = useCallback((logs: string[]) => {
    setContainerLogs(logs);
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.onreconnecting(() => console.log('Reconnecting...'));
      hubConnection.onreconnected(() => {
        console.log('Reconnected');
      });

      hubConnection.on('SendContainerLog', handleContainerLog);
      hubConnection.on('SendContainerLogsBatch', handleContainerLogsBatch);
    },
    [handleContainerLog, handleContainerLogsBatch],
  );

  const removeEventListeners = useCallback((hubConnection: HubConnection) => {
    hubConnection.off('SendContainerLog');
    hubConnection.off('SendContainerLogsBatch');
  }, []);

  useSignalRHub({
    url: `${baseUrl}/hubs/container-log`,
    accessToken,
    groupName: containerId,
    setupEventListeners,
    removeEventListeners,
  });

  return { containerLog, containerLogs };
};
