import { DockerContainerView } from '@/api/types';
import { useCallback, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { containerLogs: logs } = useContainerLogGroup(resource?.id);

  return <LogViewer logs={logs} autoScroll={true} className='pb-[20vh]' />;
};

export const useContainerLogGroup = (containerId?: string) => {
  // We use a Map to ensure uniqueness by Timestamp
  // Key: Timestamp string, Value: Log message
  const [logsMap, setLogsMap] = useState<Map<string, string>>(new Map());

  const processIncomingText = useCallback((rawText: string) => {
    setLogsMap((prev) => {
      const next = new Map(prev);
      const lines = rawText.split('\n').filter(Boolean);

      lines.forEach((line) => {
        const firstSpaceIndex = line.indexOf(' ');

        if (firstSpaceIndex !== -1) {
          const timestamp = line.substring(0, firstSpaceIndex);
          const message = line.substring(firstSpaceIndex + 1);

          next.set(timestamp, message);
        } else {
          // Fallback if timestamp is missing: use line as key (less reliable)
          next.set(line, line);
        }
      });

      // Maintain memory limit
      if (next.size > MAX_LOGS) {
        const keysToKeep = Array.from(next.keys()).slice(-MAX_LOGS);
        const limitedMap = new Map();
        keysToKeep.forEach((k) => limitedMap.set(k, next.get(k)));
        return limitedMap;
      }

      return next;
    });
  }, []);

  const handleContainerLog = useCallback(
    (log: ArrayBuffer) => {
      processIncomingText(decoder.decode(new Uint8Array(log)));
    },
    [processIncomingText],
  );

  const handleContainerLogsBatch = useCallback(
    (logs: ArrayBuffer) => {
      processIncomingText(decoder.decode(new Uint8Array(logs)));
    },
    [processIncomingText],
  );

  // Convert Map values back to an array for the Viewer
  const containerLogs = Array.from(logsMap.entries()).map(([ts, msg]) => `${ts} ${msg}`);

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
