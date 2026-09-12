import { useCallback, useState, useRef, useEffect, memo } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { LogViewer } from '@/components/custom/common';
import { normalizeContainerReference } from '@/lib/utils';

const MAX_LOGS = 5000;
const MAX_CACHED_LOG_GROUPS = 8;
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
  enableContainerFilter?: boolean;
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
    enableContainerFilter,
  }: LogsProps) => {
    const nid = normalizeContainerReference(containerId);
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
          emptyMessage="Waiting for logs…"
          autoScroll={true}
          timeStamps={true}
          allowWrap={true}
          showTimestamps={false}
          wrapLines={false}
          onClear={clearLogs}
          containerFilters={containerFilters}
          enableContainerFilter={enableContainerFilter}
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
    enableContainerFilter
  />
));
StackLogs.displayName = 'StackLogs';

interface LogEntry {
  timestamp: string;
  message: string;
}

const logsCache = new Map<string, LogEntry[]>();

const getCachedLogs = (groupName: string | undefined): LogEntry[] => {
  if (!groupName) return [];

  const cached = logsCache.get(groupName);
  if (!cached) return [];

  logsCache.delete(groupName);
  logsCache.set(groupName, cached);
  return cached;
};

const cacheLogs = (groupName: string, logs: LogEntry[]) => {
  logsCache.delete(groupName);
  logsCache.set(groupName, logs);

  while (logsCache.size > MAX_CACHED_LOG_GROUPS) {
    const oldestGroup = logsCache.keys().next().value;
    if (oldestGroup === undefined) break;
    logsCache.delete(oldestGroup);
  }
};

const mergeLogs = (current: LogEntry[], incoming: LogEntry[]): LogEntry[] => {
  if (incoming.length === 0) return current;

  incoming.sort((a, b) => a.timestamp.localeCompare(b.timestamp));
  const merged = new Array<LogEntry>(current.length + incoming.length);
  let currentIndex = 0;
  let incomingIndex = 0;
  let mergedIndex = 0;

  while (currentIndex < current.length && incomingIndex < incoming.length) {
    if (current[currentIndex].timestamp.localeCompare(incoming[incomingIndex].timestamp) <= 0) {
      merged[mergedIndex++] = current[currentIndex++];
    } else {
      merged[mergedIndex++] = incoming[incomingIndex++];
    }
  }

  while (currentIndex < current.length) merged[mergedIndex++] = current[currentIndex++];
  while (incomingIndex < incoming.length) merged[mergedIndex++] = incoming[incomingIndex++];

  return merged.length > MAX_LOGS ? merged.slice(merged.length - MAX_LOGS) : merged;
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
  const [logs, setLogs] = useState<LogEntry[]>(() => getCachedLogs(groupName));

  const logsRef = useRef<LogEntry[]>(getCachedLogs(groupName));
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

      logsRef.current = getCachedLogs(groupName);
      bufferRef.current = [];
      if (flushTimerRef.current) {
        clearTimeout(flushTimerRef.current);
        flushTimerRef.current = null;
      }

      setLogs(logsRef.current);
    }
  }, [groupName]);

  const flush = useCallback(() => {
    flushTimerRef.current = null;
    const batch = bufferRef.current.splice(0, bufferRef.current.length);
    if (batch.length === 0) return;

    const incoming: LogEntry[] = [];

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

          incoming.push({ timestamp, message });
        } else {
          incoming.push({ timestamp: new Date().toISOString(), message: line });
        }
      }
    }

    const next = mergeLogs(logsRef.current, incoming);
    logsRef.current = next;

    if (groupName) {
      cacheLogs(groupName, next);
    }

    if (mountedRef.current) {
      setLogs(next);
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
    async (hub: RealtimeConnection) => {
      try {
        await hub.invoke(hubMethodName, hubMethodArg);
      } catch (error) {
        console.error('Failed to start container logs stream', error);
      }
    },
    [hubMethodArg, hubMethodName],
  );

  const setupEventListeners = useCallback(
    (hub: RealtimeConnection) => {
      hub.on(logEventName, handleLogs);
      hub.on(logBatchEventName, handleLogs);
    },
    [handleLogs, logBatchEventName, logEventName],
  );

  const removeEventListeners = useCallback(
    (hub: RealtimeConnection) => {
      hub.off(logEventName, handleLogs);
      hub.off(logBatchEventName, handleLogs);
    },
    [handleLogs, logBatchEventName, logEventName],
  );

  useRealtimeGroup({
    groupName,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup: startLogs,
    skip: !groupName || !hubMethodArg,
  });

  const clearLogs = useCallback(() => {
    logsRef.current = [];
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
