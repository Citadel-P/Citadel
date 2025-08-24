import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { HubConnection } from '@microsoft/signalr';
import { useState, useCallback } from 'react';
const decoder = new TextDecoder('utf-8');
export const useContainerLogGroup = (containerId?: string) => {
  const [containerLogs, setContainerLogs] = useState<string[]>([]);

  const handleContainerLog = useCallback((log: ArrayBuffer) => {
    const text = decoder.decode(new Uint8Array(log));
    setContainerLogs((prev) => [...prev, text]);
  }, []);

  const handleContainerLogsBatch = useCallback((logs: ArrayBuffer) => {
    const text = decoder.decode(new Uint8Array(logs));
    setContainerLogs((prev) => [...prev, ...text.split('\n')]);
  }, []);

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

  return { containerLogs };
};
