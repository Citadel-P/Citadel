import { useCallback, useMemo, useState, useRef, useEffect } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';
import { normalizeDockerId } from '@/lib/utils';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = ({ containerId }: { containerId: string | undefined }) => {
  const nid = normalizeDockerId(containerId);
  const { containerLogs: logs, clearLogs } = useContainerLogGroup(nid);

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

const useContainerLogGroup = (containerId?: string) => {
  // We use a Map to ensure uniqueness by Timestamp
  // Key: Timestamp string, Value: Log message
  const [logsMap, setLogsMap] = useState<Map<string, string>>(new Map());
  const processRef = useRef<(rawText: string) => void>(() => {});

  useEffect(() => {
    setLogsMap(new Map());
  }, [containerId]);

  processRef.current = (rawText: string) => {
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
          // Fallback for lines without standard Docker timestamps
          next.set(`${new Date().getTime()}-${Math.random()}`, line);
        }
      });

      if (next.size > MAX_LOGS) {
        const entries = Array.from(next.entries()).slice(-MAX_LOGS);
        return new Map(entries);
      }
      return next;
    });
  };

  const handleLogs = useCallback((data: ArrayBuffer) => {
    const text = decoder.decode(new Uint8Array(data));
    processRef.current(text);
  }, []);

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.on('SendContainerLogs', handleLogs);
      hub.on('SendContainerLogsBatch', handleLogs);
    },
    [handleLogs],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off('SendContainerLogs', handleLogs);
      hub.off('SendContainerLogsBatch', handleLogs);
    },
    [handleLogs],
  );

  useSignalRGroup({
    groupName: `container-log:${containerId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !containerId,
  });

  const clearLogs = useCallback(() => setLogsMap(new Map()), []);

  const containerLogs = useMemo(
    () =>
      Array.from(logsMap.entries())
        .map(([ts, msg]) => ({
          timestamp: ts,
          message: msg,
        }))
        .sort((a, b) => a.timestamp.localeCompare(b.timestamp)), // Ensure chronological order
    [logsMap],
  );

  return { containerLogs, clearLogs };
};
