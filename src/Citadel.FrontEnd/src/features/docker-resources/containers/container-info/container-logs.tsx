import { useCallback, useState, useRef, useEffect, memo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';
import { normalizeDockerId } from '@/lib/utils';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');
const DOCKER_TIMESTAMP_ONLY_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;

type LogsProps = {
  hubMethodName: string;
  containerId?: string | undefined;
  deploymentId?: string | undefined;
  stackId?: string | undefined;
  groupName?: string | undefined;
  logEventName?: string;
  logBatchEventName?: string;
  containerFilters?: string[];
};

const Logs = memo(
  ({
    hubMethodName,
    containerId,
    deploymentId,
    stackId,
    groupName,
    logEventName = 'SendContainerLogs',
    logBatchEventName = 'SendContainerLogsBatch',
    containerFilters,
  }: LogsProps) => {
    const nid = normalizeDockerId(containerId);
    const effectiveGroupName = groupName ?? (nid ? `container-log:${nid}` : undefined);
    const hubMethodArg = deploymentId ?? stackId ?? nid;
    const { containerLogs: logs, clearLogs } = useContainerLogGroup({
      hubMethodName,
      hubMethodArg,
      groupName: effectiveGroupName,
      logEventName,
      logBatchEventName,
    });

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
          containerFilters={containerFilters}
          className="pb-[20vh]"
        />
      </div>
    );
  },
);
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

export const StackLogs = memo(({ stackId, containers }: { stackId: string; containers?: string[] }) => (
  <Logs
    hubMethodName="StartStackLogs"
    stackId={stackId}
    groupName={`stack-log:${stackId}`}
    logEventName="SendStackLogs"
    logBatchEventName="SendStackLogsBatch"
    containerFilters={containers}
  />
));
StackLogs.displayName = 'StackLogs';

interface LogEntry {
  timestamp: string;
  message: string;
}

type LogEntryMap = Map<string, LogEntry>;

const logsCache = new Map<string, LogEntryMap>();

const getCachedLogs = (groupName: string | undefined): LogEntryMap => {
  const cached = groupName ? logsCache.get(groupName) : undefined;
  return cached ? new Map(cached) : new Map();
};

const sortLogs = (logsMap: LogEntryMap) => {
  const arr = Array.from(logsMap.values());
  arr.sort((a, b) => a.timestamp.localeCompare(b.timestamp));
  return arr;
};

export const useContainerLogGroup = ({
  hubMethodName,
  hubMethodArg,
  groupName,
  logEventName,
  logBatchEventName,
}: {
  hubMethodName: string;
  hubMethodArg?: string | undefined;
  groupName?: string | undefined;
  logEventName: string;
  logBatchEventName: string;
}) => {
  const [logs, setLogs] = useState<LogEntry[]>(() => sortLogs(getCachedLogs(groupName)));

  const logsMapRef = useRef<LogEntryMap>(getCachedLogs(groupName));
  const bufferRef = useRef<string[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevGroupNameRef = useRef<string | undefined>(groupName);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Safe reset when the resource ID actually changes
  useEffect(() => {
    if (groupName !== prevGroupNameRef.current) {
      prevGroupNameRef.current = groupName;

      logsMapRef.current = getCachedLogs(groupName);
      bufferRef.current = [];
      if (flushTimerRef.current) {
        clearTimeout(flushTimerRef.current);
        flushTimerRef.current = null;
      }

      setLogs(sortLogs(logsMapRef.current));
    }
  }, [groupName]);

  const flush = useCallback(() => {
    flushTimerRef.current = null;
    const batch = bufferRef.current.splice(0, bufferRef.current.length);
    if (batch.length === 0) return;

    const next = new Map(logsMapRef.current);
    let sequence = next.size;

    for (const rawText of batch) {
      for (const line of rawText.split('\n')) {
        if (!line) continue;
        const trimmedLine = line.trim();
        if (DOCKER_TIMESTAMP_ONLY_RE.test(trimmedLine)) continue;

        const firstSpaceIndex = line.indexOf(' ');
        const firstToken = firstSpaceIndex !== -1 ? line.substring(0, firstSpaceIndex) : '';
        const hasTimestampPrefix = firstToken.includes('T') || /^\d{2}:\d{2}:\d{2}/.test(firstToken);

        if (firstSpaceIndex !== -1 && hasTimestampPrefix) {
          const timestamp = line.substring(0, firstSpaceIndex);
          const message = line.substring(firstSpaceIndex + 1);
          if (!message.trim()) continue;

          next.set(`${timestamp}-${sequence++}`, { timestamp, message });
        } else {
          // Fallback for malformed lines
          next.set(`${Date.now()}-${sequence++}`, { timestamp: `${Date.now()}`, message: line });
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

    if (groupName) {
      logsCache.set(groupName, new Map(logsMapRef.current));
    }

    if (mountedRef.current) {
      setLogs(sortLogs(logsMapRef.current));
    }
  }, [groupName]);

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
        await hub.invoke(hubMethodName, hubMethodArg);
      } catch (error) {
        console.error('Failed to start container logs stream', error);
      }
    },
    [hubMethodArg, hubMethodName],
  );

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.on(logEventName, handleLogs);
      hub.on(logBatchEventName, handleLogs);
    },
    [handleLogs, logBatchEventName, logEventName],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off(logEventName, handleLogs);
      hub.off(logBatchEventName, handleLogs);
    },
    [handleLogs, logBatchEventName, logEventName],
  );

  useSignalRGroup({
    groupName,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup: startLogs,
    skip: !groupName || !hubMethodArg,
  });

  const clearLogs = useCallback(() => {
    logsMapRef.current = new Map();
    bufferRef.current = [];
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    if (groupName) {
      logsCache.delete(groupName);
    }
    setLogs([]);
  }, [groupName]);

  return { containerLogs: logs, clearLogs };
};
