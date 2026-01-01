import CodeHighlight from '@/components/custom/code-highlight';
import { DockerContainerView } from '@/api/types';
import { useCallback, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { containerLogs: logs } = useContainerLogGroup(resource?.id);

  return <CodeHighlight code={logs?.length === 0 ? 'No logs' : logs!.join('\n')} language="tsx" autoScroll />;
};

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
