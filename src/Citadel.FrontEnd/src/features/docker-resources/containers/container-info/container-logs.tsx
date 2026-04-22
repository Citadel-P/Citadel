import { useCallback, useState, useRef, useEffect, memo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';
import { normalizeDockerId } from '@/lib/utils';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = memo(({ containerId }: { containerId: string | undefined }) => {
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
});
ContainerLogs.displayName = 'ContainerLogs';

interface LogEntry {
  timestamp: string;
  message: string;
}

const useContainerLogGroup = (containerId?: string) => {
  // Mutable refs hold the real data; React only knows about "version".
  const logsMapRef = useRef<Map<string, string>>(new Map());
  const logsArrayRef = useRef<LogEntry[]>([]);
  const bufferRef = useRef<string[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [_, setVersion] = useState(0);

  // Reset everything when the container changes.
  useEffect(() => {
    logsMapRef.current = new Map();
    logsArrayRef.current = [];
    bufferRef.current = [];
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    setVersion((v) => v + 1);
  }, [containerId]);

  // Flush buffered raw text into the Map, trim to MAX_LOGS, sort, and bump version.
  const flush = useCallback(() => {
    flushTimerRef.current = null;
    const batch = bufferRef.current.splice(0, bufferRef.current.length);
    if (batch.length === 0) return;

    const next = new Map(logsMapRef.current);

    for (const rawText of batch) {
      for (const line of rawText.split('\n')) {
        if (!line) continue;
        const firstSpaceIndex = line.indexOf(' ');
        if (firstSpaceIndex !== -1) {
          const timestamp = line.substring(0, firstSpaceIndex);
          const message = line.substring(firstSpaceIndex + 1);
          next.set(timestamp, message);
        } else {
          next.set(`${Date.now()}-${Math.random()}`, line);
        }
      }
    }

    if (next.size > MAX_LOGS) {
      const entries = Array.from(next.entries()).slice(-MAX_LOGS);
      logsMapRef.current = new Map(entries);
    } else {
      logsMapRef.current = next;
    }

    // Build sorted array once, outside React render phase.
    const arr = new Array<LogEntry>(logsMapRef.current.size);
    let i = 0;
    for (const [ts, msg] of logsMapRef.current) {
      arr[i++] = { timestamp: ts, message: msg };
    }
    arr.sort((a, b) => a.timestamp.localeCompare(b.timestamp));
    logsArrayRef.current = arr;

    setVersion((v) => v + 1);
  }, []);

  // Incoming SignalR data -> buffer-> schedule flush (max once per 50 ms).
  const handleLogs = useCallback(
    (data: ArrayBuffer) => {
      const text = decoder.decode(new Uint8Array(data));
      bufferRef.current.push(text);
      if (!flushTimerRef.current) {
        flushTimerRef.current = setTimeout(flush, 50);
      }
    },
    [flush],
  );

  const startLogs = useCallback(
    async (hub: HubConnection) => {
      if (!containerId) return;
      try {
        await hub.invoke('StartContainerLogs', containerId);
      } catch (error) {
        console.error('Failed to start container logs stream', error);
      }
    },
    [containerId],
  );

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
    onJoinedGroup: startLogs,
    skip: !containerId,
  });

  const clearLogs = useCallback(() => {
    logsMapRef.current = new Map();
    logsArrayRef.current = [];
    bufferRef.current = [];
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    setVersion((v) => v + 1);
  }, []);

  // Return the ref array directly. Because "version" changed, consumers re-render with the latest data.
  return { containerLogs: logsArrayRef.current, clearLogs };
};
