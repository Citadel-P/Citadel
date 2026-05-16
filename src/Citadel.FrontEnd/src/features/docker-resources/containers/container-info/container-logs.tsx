import { useCallback, useState, useRef, useEffect, memo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';
import { normalizeDockerId } from '@/lib/utils';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

type LogsProps = {
  hubMethodName: string;
  containerId?: string | undefined;
  deploymentId?: string | undefined;
};

const Logs = memo(({ hubMethodName, containerId, deploymentId }: LogsProps) => {
  const nid = normalizeDockerId(containerId);
  const { containerLogs: logs, clearLogs } = useContainerLogGroup(hubMethodName, nid, deploymentId);

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
Logs.displayName = 'Logs';

export const ContainerLogs = memo(({ containerId }: { containerId: string }) => (
  <Logs hubMethodName="StartContainerLogs" containerId={containerId} />
));
ContainerLogs.displayName = 'ContainerLogs';

export const DeploymentLogs = memo(
  ({ deploymentId, containerId }: { deploymentId: string; containerId: string | undefined }) => (
    <Logs hubMethodName="StartDeploymentLogs" containerId={containerId} deploymentId={deploymentId} />
  ),
);
DeploymentLogs.displayName = 'DeploymentLogs';

interface LogEntry {
  timestamp: string;
  message: string;
}

export const useContainerLogGroup = (
  hubMethodName: string,
  containerId: string | undefined,
  deploymentId?: string | undefined,
) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const logsMapRef = useRef<Map<string, string>>(new Map());
  const bufferRef = useRef<string[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevContainerIdRef = useRef<string | undefined>(containerId);

  // Safe reset when the resource ID actually changes
  useEffect(() => {
    if (containerId !== prevContainerIdRef.current) {
      prevContainerIdRef.current = containerId;

      logsMapRef.current = new Map();
      bufferRef.current = [];
      if (flushTimerRef.current) {
        clearTimeout(flushTimerRef.current);
        flushTimerRef.current = null;
      }

      setLogs([]);
    }
  }, [containerId]);

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
          // Fallback for malformed lines
          next.set(`${Date.now()}-${Math.random()}`, line);
        }
      }
    }

    // Trim to MAX_LOGS
    if (next.size > MAX_LOGS) {
      const entries = Array.from(next.entries()).slice(-MAX_LOGS);
      logsMapRef.current = new Map(entries);
    } else {
      logsMapRef.current = next;
    }

    // Build sorted array
    const arr = new Array<LogEntry>(logsMapRef.current.size);
    let i = 0;
    for (const [ts, msg] of logsMapRef.current) {
      arr[i++] = { timestamp: ts, message: msg };
    }
    arr.sort((a, b) => a.timestamp.localeCompare(b.timestamp));

    setLogs(arr);
  }, []);

  // max 50ms batching
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
      try {
        await hub.invoke(hubMethodName, deploymentId ?? containerId);
      } catch (error) {
        console.error('Failed to start container logs stream', error);
      }
    },
    [containerId, hubMethodName, deploymentId],
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
    bufferRef.current = [];
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    setLogs([]);
  }, []);

  return { containerLogs: logs, clearLogs };
};
