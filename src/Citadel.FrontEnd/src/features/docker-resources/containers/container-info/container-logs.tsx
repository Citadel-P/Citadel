import { DockerContainerView } from '@/api/types';
import { useCallback, useMemo, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { LogViewer } from '@/components/custom/common';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Eraser } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

const MAX_LOGS = 5000;
const decoder = new TextDecoder('utf-8');

export const ContainerLogs = ({ resource }: { resource: DockerContainerView | undefined }) => {
  const { containerLogs: logs, clearLogs } = useContainerLogGroup(resource?.id);
  const [showTimestamps, setShowTimestamps] = useState(false);
  const [wrapLines, setWrapLines] = useState(false);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-4 pl-2 justify-between">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2 ">
            <Label className="text-xs text-foreground/80 font-normal" htmlFor="timestamps">
              Timestamps
            </Label>
            <Switch checked={showTimestamps} id="timestamps" onCheckedChange={setShowTimestamps} />
          </div>
          <div className="flex items-center gap-2 ">
            <Label className="text-xs text-foreground/80 font-normal" htmlFor="wrap-lines">
              Wrap Lines
            </Label>
            <Switch checked={wrapLines} id="wrap-lines" onCheckedChange={setWrapLines} />
          </div>
        </div>

        <div className="flex items-center">
          <TooltipProvider delayDuration={200}>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="icon-sm" onClick={clearLogs} className="rounded-full">
                  <Eraser className="h-3.5 w-3.5 " />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Clear Console</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      </div>
      <LogViewer
        logs={logs}
        autoScroll={true}
        showTimestamps={showTimestamps}
        wrapLines={wrapLines}
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
