import { DockerContainerView } from '@/api/types';
import { useCallback, useMemo, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { containerLogs: logs, clearLogs } = useContainerLogGroup(resource?.id);

  return (
    <div className="flex flex-col gap-3">
      <LogViewer
        logs={logs}
        autoScroll={true}
        timeStamps={true}
        allowWrap={true}
        showTimestamps={false}
        wrapLines={false}
        onClear={clearLogs}
        className="pb-[20vh]"
      />
    </div>
  );
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
          next.set(new Date().toISOString(), line);
        }
      });

      if (next.size > MAX_LOGS) {
        const entries = Array.from(next.entries()).slice(-MAX_LOGS);
        return new Map(entries);
      }
      return next;
    });
  }, []);

  const clearLogs = useCallback(() => {
    setLogsMap(new Map());
  }, []);

  const handleContainerLogs = useCallback(
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

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.on('SendContainerLogs', handleContainerLogs);
      hub.on('SendContainerLogsBatch', handleContainerLogsBatch);
    },
    [handleContainerLogs, handleContainerLogsBatch],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off('SendContainerLogs', handleContainerLogs);
      hub.off('SendContainerLogsBatch', handleContainerLogsBatch);
    },
    [handleContainerLogs, handleContainerLogsBatch],
  );

  useSignalRGroup({
    groupName: `container-log:${containerId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !containerId,
  });

  const containerLogs = useMemo(
    () =>
      Array.from(logsMap.entries()).map(([ts, msg]) => ({
        timestamp: ts,
        message: msg,
      })),
    [logsMap],
  );

  return { containerLogs, clearLogs };
};
