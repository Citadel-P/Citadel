import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { HubConnection } from '@microsoft/signalr';
import { useState, useCallback } from 'react';

export const useContainerLogGroup = (containerId?: string) => {
  const [containerLog, setContainerLog] = useState<string | undefined>();
  const [containerLogs, setContainerLogs] = useState<string[]>([]);

  const handleContainerLog = useCallback((log: string) => setContainerLog(log), []);
  const handleContainerLogsBatch = useCallback((logs: string[]) => setContainerLogs(logs), []);

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.on('SendContainerLog', handleContainerLog);
      hub.on('SendContainerLogsBatch', handleContainerLogsBatch);
    },
    [handleContainerLog, handleContainerLogsBatch],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off('SendContainerLog', handleContainerLog);
      hub.off('SendContainerLogsBatch', handleContainerLogsBatch);
    },
    [handleContainerLog, handleContainerLogsBatch],
  );

  useSignalRGroup({
    groupName: `container-log:${containerId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !containerId,
  });

  return { containerLog, containerLogs };
};
