import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { useLocalStorage } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import Convert from 'ansi-to-html';
import {
  Check,
  CircleCheck,
  CircleX,
  Eraser,
  Funnel,
  Info,
  LucideIcon,
  Timer,
  TriangleAlert,
  WrapText,
} from 'lucide-react';
import { memo, useCallback, useDeferredValue, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { getContainerSeriesColor } from './container-series-colors';

export interface LogEntry {
  timestamp?: string;
  message: string;
  severity?: LogSeverity;
}

interface RenderedLogEntry extends LogEntry {
  html: string;
  containerName?: string;
  severity?: LogSeverity;
}

export type LogSeverity = 'info' | 'success' | 'warning' | 'error';

interface LogViewerProps {
  logs: string | string[] | LogEntry[];
  emptyMessage?: string;
  autoScroll?: boolean;
  className?: string;
  showTimestamps?: boolean;
  wrapLines?: boolean;
  timeStamps?: boolean;
  allowWrap?: boolean;
  onClear?: () => void;
  containerFilters?: string[];
  enableContainerFilter?: boolean;
}

interface QuickActionProps {
  label: string;
  icon: React.ReactNode;
  active?: boolean;
  disabled?: boolean;
  side?: 'left' | 'top' | 'right' | 'bottom';
  onClick: () => void;
}

const QuickAction = ({ label, icon, active, disabled, side = 'left', onClick }: QuickActionProps) => (
  <TooltipProvider delayDuration={200}>
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          aria-label={label}
          disabled={disabled}
          size="icon-sm"
          variant={active ? 'secondary' : 'outline'}
          className={'rounded-full h-7 w-7 shadow-sm'}
          onClick={onClick}>
          {icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </Tooltip>
  </TooltipProvider>
);

const ansiConverter = new Convert({
  newline: false,
  escapeXML: true,
  stream: true,
});

// Memoised row so React skips it when only scroll position changes.
const LogRow = memo(
  ({ log, showTimestamps, wrapLines }: { log: RenderedLogEntry; showTimestamps: boolean; wrapLines: boolean }) => {
    const containerColor = log.containerName ? getContainerSeriesColor(log.containerName) : undefined;
    const severity = log.severity ? logSeverityConfig[log.severity] : undefined;
    const SeverityIcon = severity?.Icon;

    return (
      <div className={cn('flex gap-2 min-h-[1.2rem]', wrapLines ? 'whitespace-pre-wrap break-all' : 'whitespace-pre')}>
        {showTimestamps && log.timestamp && (
          <span className="text-foreground/30 shrink-0 select-none tabular-nums">
            {log.timestamp.includes('T') ? log.timestamp.split('T')[1].slice(0, 8) : log.timestamp}
          </span>
        )}
        {log.containerName && (
          <span className={cn('shrink-0 select-none font-semibold', containerColor?.text)}>[{log.containerName}]</span>
        )}
        {SeverityIcon && <SeverityIcon className={cn('mt-0.5 size-3.5 shrink-0', severity.iconClassName)} />}
        <span className={severity?.textClassName} dangerouslySetInnerHTML={{ __html: log.html }} />
      </div>
    );
  },
);
LogRow.displayName = 'LogRow';

const logSeverityConfig = {
  info: {
    Icon: Info,
    iconClassName: 'text-info',
    textClassName: 'text-info',
  },
  success: {
    Icon: CircleCheck,
    iconClassName: 'text-success',
    textClassName: 'text-success',
  },
  warning: {
    Icon: TriangleAlert,
    iconClassName: 'text-warning',
    textClassName: 'text-warning',
  },
  error: {
    Icon: CircleX,
    iconClassName: 'text-destructive',
    textClassName: 'text-destructive',
  },
} satisfies Record<LogSeverity, { Icon: LucideIcon; iconClassName: string; textClassName: string }>;

const parseLogContainerLabel = (message: string): { containerName?: string; message: string } => {
  const match = message.match(/^\[([^\]]+)\]\s?(.*)$/s);
  if (!match) return { message };

  return {
    containerName: match[1],
    message: match[2] ?? '',
  };
};

function LogContainerFilter({
  containers,
  selectedContainers,
  onToggle,
}: {
  containers: string[];
  selectedContainers: string[];
  onToggle: (container: string) => void;
}) {
  const selectedSet = useMemo(() => new Set(selectedContainers), [selectedContainers]);
  const active = selectedContainers.length !== containers.length;

  return (
    <Popover>
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <PopoverTrigger asChild>
              <Button
                aria-label="Container filter"
                size="icon-sm"
                variant={active ? 'secondary' : 'outline'}
                className="h-7 w-7 rounded-full shadow-sm">
                <Funnel className="h-3.5 w-3.5" />
              </Button>
            </PopoverTrigger>
          </TooltipTrigger>
          <TooltipContent side="left">Container filter</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <PopoverContent align="end" side="left" className="w-64 p-2 bg-background">
        <div className="px-2 pb-2 text-xs font-medium text-muted-foreground">Containers</div>
        <div className="max-h-64 overflow-auto">
          {containers.map((container) => {
            const color = getContainerSeriesColor(container);
            const selected = selectedSet.has(container);
            return (
              <button
                type="button"
                key={container}
                className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                onClick={() => onToggle(container)}>
                <span
                  className={cn(
                    'flex size-4 shrink-0 items-center justify-center border',
                    selected && 'bg-primary text-primary-foreground',
                  )}>
                  {selected && <Check className="size-3" />}
                </span>
                <span className={cn('h-2 w-2 shrink-0 rounded-full', color.dot)} />
                <span className="truncate" title={container}>
                  {container}
                </span>
              </button>
            );
          })}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// Simple virtual-list constants.
const ITEM_HEIGHT = 20; // px — matches min-h-[1.2rem] + text-xs
const VIEWPORT_HEIGHT = 600; // px — matches max-h-[600px]
const OVERSCAN = 10;

export const LogViewer = memo(
  ({
    logs,
    emptyMessage = 'No logs available...',
    autoScroll = true,
    className,
    showTimestamps: initialShowTimestamps = false,
    wrapLines: initialWrapLines = true,
    timeStamps: enableTimestamps = false,
    allowWrap = false,
    onClear,
    containerFilters,
    enableContainerFilter,
  }: LogViewerProps) => {
    const scrollRef = useRef<HTMLDivElement>(null);
    const [isAtBottom, setIsAtBottom] = useState(true);
    const [showTimestamps, setShowTimestamps] = useLocalStorage('log-viewer-show-timestamps', initialShowTimestamps);
    const [wrapLines, setWrapLines] = useLocalStorage('log-viewer-wrap-lines', initialWrapLines);
    const [excludedContainers, setExcludedContainers] = useState<string[]>([]);
    const [scrollTop, setScrollTop] = useState(0);

    // Defer heavy log updates so toggles / scroll stay responsive.
    const deferredLogs = useDeferredValue(logs);
    const containerFilteringEnabled = enableContainerFilter ?? Boolean(containerFilters?.length);

    const normalizedLogs = useMemo((): LogEntry[] => {
      if (!deferredLogs) return [];

      if (typeof deferredLogs === 'string') {
        return parseAnsiTerminalStream(deferredLogs).map((line) => ({ message: line }));
      }

      return deferredLogs.flatMap((log) => {
        if (typeof log === 'object' && log !== null) {
          return parseAnsiTerminalStream(log.message).map((line) => ({
            timestamp: log.timestamp,
            message: line,
            severity: log.severity,
          }));
        }

        return parseAnsiTerminalStream(String(log)).map((line) => ({ message: line }));
      });
    }, [deferredLogs]);

    const availableContainers = useMemo(() => {
      if (!containerFilteringEnabled) return [];

      const fromProps = (containerFilters ?? []).map((container) => container.trim()).filter(Boolean);
      if (fromProps.length > 0) {
        return Array.from(new Set(fromProps)).sort((a, b) => a.localeCompare(b));
      }

      const fromLogs = normalizedLogs
        .map((log) => parseLogContainerLabel(log.message).containerName)
        .filter((container): container is string => Boolean(container));

      return Array.from(new Set(fromLogs)).sort((a, b) => a.localeCompare(b));
    }, [containerFilteringEnabled, containerFilters, normalizedLogs]);

    const selectedContainers = useMemo(() => {
      const excluded = new Set(excludedContainers);
      return availableContainers.filter((container) => !excluded.has(container));
    }, [availableContainers, excludedContainers]);

    const selectedContainerSet = useMemo(() => new Set(selectedContainers), [selectedContainers]);

    const toggleContainerFilter = useCallback((container: string) => {
      setExcludedContainers((previous) =>
        previous.includes(container) ? previous.filter((excluded) => excluded !== container) : [...previous, container],
      );
    }, []);

    // Convert ANSI once per unique message, then cache.
    const renderedLogs = useMemo(() => {
      const ansiCache = new Map<string, string>();

      return normalizedLogs.map((log): RenderedLogEntry => {
        const parsed = containerFilteringEnabled ? parseLogContainerLabel(log.message) : { message: log.message };
        const formatted = formatLogMessage(parsed.message);
        const message = formatted.message;
        let html = ansiCache.get(message);
        if (html === undefined) {
          html = ansiConverter.toHtml(message);
          ansiCache.set(message, html);
        }
        return {
          ...log,
          message,
          containerName: parsed.containerName,
          html,
          severity: log.severity ?? formatted.severity,
        };
      });
    }, [containerFilteringEnabled, normalizedLogs]);

    const filteredLogs = useMemo(() => {
      if (!containerFilteringEnabled) return renderedLogs;
      if (availableContainers.length === 0) return renderedLogs;

      return renderedLogs.filter((log) => log.containerName && selectedContainerSet.has(log.containerName));
    }, [availableContainers.length, containerFilteringEnabled, renderedLogs, selectedContainerSet]);

    // Virtual window.
    const virtual = useMemo(() => {
      const totalHeight = filteredLogs.length * ITEM_HEIGHT;
      const start = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - OVERSCAN);
      const end = Math.min(filteredLogs.length, Math.ceil((scrollTop + VIEWPORT_HEIGHT) / ITEM_HEIGHT) + OVERSCAN);
      return {
        totalHeight,
        start,
        end,
        paddingTop: start * ITEM_HEIGHT,
        paddingBottom: (filteredLogs.length - end) * ITEM_HEIGHT,
        items: filteredLogs.slice(start, end),
      };
    }, [filteredLogs, scrollTop]);

    const handleScroll = useCallback(() => {
      if (!scrollRef.current) return;
      const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
      setScrollTop(scrollTop);
      setIsAtBottom(scrollHeight - scrollTop <= clientHeight + 50);
    }, []);

    useLayoutEffect(() => {
      if (autoScroll && isAtBottom && scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    }, [filteredLogs.length, autoScroll, isAtBottom, wrapLines, showTimestamps]);

    return (
      <div className="relative flex flex-col h-full border rounded-md overflow-hidden">
        {(enableTimestamps || availableContainers.length > 0 || allowWrap || onClear) && (
          <div className="absolute top-2 right-2 flex flex-col gap-2 z-10">
            {enableTimestamps && (
              <QuickAction
                label="Timestamps"
                icon={<Timer className="h-3.5 w-3.5" />}
                active={showTimestamps}
                onClick={() => setShowTimestamps(!showTimestamps)}
              />
            )}
            {availableContainers.length > 0 && (
              <LogContainerFilter
                containers={availableContainers}
                selectedContainers={selectedContainers}
                onToggle={toggleContainerFilter}
              />
            )}
            {allowWrap && (
              <QuickAction
                label="Wrap Lines"
                icon={<WrapText className="h-3.5 w-3.5" />}
                active={wrapLines}
                onClick={() => setWrapLines(!wrapLines)}
              />
            )}
            {onClear && (
              <QuickAction label="Clear Console" icon={<Eraser className="h-3.5 w-3.5" />} onClick={onClear} />
            )}
          </div>
        )}

        <div
          ref={scrollRef}
          onScroll={handleScroll}
          className={cn(
            'p-4 max-h-150 rounded-sm text-xs inline-block w-full overflow-auto bg-transparent',
            className,
          )}>
          {renderedLogs.length > 0 ? (
            <>
              {/* Top spacer */}
              <div style={{ height: virtual.paddingTop }} aria-hidden />
              {virtual.items.map((log, idx) => (
                <LogRow key={virtual.start + idx} log={log} showTimestamps={showTimestamps} wrapLines={wrapLines} />
              ))}
              {/* Bottom spacer */}
              <div style={{ height: virtual.paddingBottom }} aria-hidden />
            </>
          ) : (
            <div className="text-zinc-600 italic">{emptyMessage}</div>
          )}
        </div>
      </div>
    );
  },
);
LogViewer.displayName = 'LogViewer';
function parseAnsiTerminalStream(rawText: string): string[] {
  if (!rawText) return [];

  // eslint-disable-next-line no-control-regex
  let cleanText = rawText.replace(/\x1b\[\?25[lh]/g, '').replace(/\?25[lh]/g, '');

  // Heal stripped Docker Compose cursor controls (e.g., AG[+] or G[+])
  // We capture any number of 'A' tokens (Cursor Up) followed by 'G' (Carriage Return)
  // right before the '[+]' progress block, and restore them to standard terminal controls.
  cleanText = cleanText.replace(/(^|[^\d[])(A*)G(?=\s*\[\+\])/g, (_, prefix, cursorUpChars) => {
    const structuralUps = '\x1b[A'.repeat(cursorUpChars.length);
    return prefix + structuralUps + '\r';
  });
  cleanText = collapseDockerComposeFrames(cleanText);

  const lines: string[] = [];
  let cursorLine = 0;

  // eslint-disable-next-line no-control-regex
  const tokens = cleanText.split(/(\r|\n|\x1b\[\d*A|\x1b\[\d*G)/);

  for (const token of tokens) {
    if (!token) continue;

    if (token === '\n') {
      cursorLine++;
      if (cursorLine >= lines.length) {
        lines.push('');
      }
    } else if (token === '\r' || (token.startsWith('\x1b[') && token.endsWith('G'))) {
      if (lines[cursorLine] !== undefined) {
        lines[cursorLine] = '';
      }
    } else if (token.startsWith('\x1b[') && token.endsWith('A')) {
      // eslint-disable-next-line no-control-regex
      const match = token.match(/\x1b\[(\d+)A/);
      const count = match ? parseInt(match[1], 10) : 1;
      cursorLine = Math.max(0, cursorLine - count);
    } else {
      if (lines[cursorLine] === undefined) {
        while (lines.length <= cursorLine) lines.push('');
      }
      lines[cursorLine] = token;
    }
  }

  while (lines.length > 0 && lines[lines.length - 1].trim() === '') {
    lines.pop();
  }

  return lines.map((line) => line.trimEnd());
}

function collapseDockerComposeFrames(value: string) {
  const output: string[] = [];
  let composeHeader: string | undefined;
  let composeRows = new Map<string, string>();

  const flushComposeFrame = () => {
    if (!composeHeader) return;
    output.push(composeHeader);
    output.push(...composeRows.values());
    composeHeader = undefined;
    composeRows = new Map<string, string>();
  };

  for (const rawLine of value.split('\n')) {
    const line = removeCursorControls(rawLine).trimEnd();
    const plainLine = stripAnsiCodes(line).trim();
    if (!plainLine) {
      continue;
    }

    if (plainLine.startsWith('[+]')) {
      composeHeader = line;
      continue;
    }

    const composeResourceKey = composeHeader ? getDockerComposeResourceKey(plainLine) : undefined;
    if (composeResourceKey) {
      composeRows.set(composeResourceKey, line);
      continue;
    }

    flushComposeFrame();
    output.push(line);
  }

  flushComposeFrame();
  return output.join('\n');
}

function getDockerComposeResourceKey(value: string) {
  const match = value.match(/\b(Container|Network|Volume|Image|Service)\s+(\S+)/);
  return match ? `${match[1]} ${match[2]}` : undefined;
}

function formatLogMessage(message: string): { message: string; severity?: LogSeverity } {
  const plainMessage = stripAnsiCodes(message).trim();
  if (isTaskSuccessMessage(plainMessage)) {
    return { message, severity: 'success' };
  }
  const levelMatch = plainMessage.match(/\blevel=(debug|info|warning|warn|error|fatal|panic)\b/i);
  if (!levelMatch) return { message };

  const msg = parseLogfmtMessageValue(plainMessage);
  if (!msg) return { message };

  const level = levelMatch[1].toLowerCase();
  if (level === 'warning' || level === 'warn') {
    return { message: msg, severity: 'warning' };
  }

  if (level === 'error' || level === 'fatal' || level === 'panic') {
    return { message: msg, severity: 'error' };
  }

  if (level === 'info') {
    return { message: msg, severity: 'info' };
  }

  return { message: msg };
}

function isTaskSuccessMessage(message: string): boolean {
  return (
    message === 'Stack is now running.' ||
    message === 'Deployment is now running.' ||
    message === 'Stack rolled back successfully.' ||
    message === 'Stack applied successfully.'
  );
}

function parseLogfmtMessageValue(value: string): string | undefined {
  const quoted = value.match(/\bmsg="((?:\\.|[^"\\])*)"/);
  if (quoted) {
    return quoted[1].replace(/\\"/g, '"').replace(/\\\\/g, '\\').replace(/\\n/g, '\n').replace(/\\t/g, '\t');
  }

  const unquoted = value.match(/\bmsg=([^\s].*)$/);
  return unquoted?.[1]?.trim();
}

function removeCursorControls(value: string) {
  return (
    value
      // eslint-disable-next-line no-control-regex
      .replace(/\x1b\[\d*[AG]/g, '')
      .replace(/\r/g, '')
      // Some malformed stream chunks can leave a stripped cursor-column sequence as ESC[0[+].
      // eslint-disable-next-line no-control-regex
      .replace(/\x1b\[\d*(?=\[\+\])/, '')
  );
}

function stripAnsiCodes(value: string) {
  // eslint-disable-next-line no-control-regex
  return value.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '');
}
