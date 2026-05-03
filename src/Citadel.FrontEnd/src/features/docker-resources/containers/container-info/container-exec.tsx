import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useLayoutContext } from '@/lib/context/layout-context';
import { normalizeDockerId } from '@/lib/utils';
import { nanoid } from 'nanoid';
import '@xterm/xterm/css/xterm.css';
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Power, PowerOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ExecTarget } from '@/api/types';

const SHELLS = [
  { label: 'bash', value: 'bash' },
  { label: 'sh', value: 'sh' },
];

export const ContainerExec = ({ containerId, disabled, target }: { containerId?: string; disabled?: boolean; target: ExecTarget }) => {
  const nid = normalizeDockerId(containerId);
  const { terminalRef, isLoading, isConnected, shell, setShell, toggleConnection } = useContainerExecTerminal(
    nid,
    disabled,
    target,
  );

  return (
    <div className="flex flex-col w-full h-[60vh]">
      <div className="flex items-center justify-between bg-secondary/20 p-2 rounded-t-md ">
        <div className="flex flex-row gap-4">
          <Select onValueChange={(e) => setShell(e as 'bash' | 'sh')} value={shell} disabled={isConnected || isLoading}>
            <SelectTrigger className="w-full min-w-32 max-h-[32px] bg-background rounded-sm shadow-xs">
              <SelectValue placeholder="Select a shell" />
            </SelectTrigger>
            <SelectContent className="bg-background">
              <SelectGroup>
                {SHELLS.map((s) => (
                  <SelectItem key={s.value} value={s.value}>
                    {s.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>

          <Button
            disabled={isLoading}
            variant="outline"
            className="rounded-sm h-[32px] shadow-xs text-sm font-normal"
            onClick={toggleConnection}>
            {isConnected ? (
              <span className="flex items-center gap-2">
                Disconnect
                <PowerOff className="h-3 w-3" />
              </span>
            ) : (
              <span className="flex items-center gap-2">
                Connect
                <Power className="h-3 w-3" />
              </span>
            )}
          </Button>
        </div>
      </div>
      {!disabled && <div ref={terminalRef} className="flex-1 w-full" />}
    </div>
  );
};

const THEMES = {
  light: { background: '#f7f8f9', foreground: '#24292e', cursor: '#24292e', selectionBackground: '#c8d9fa' },
  dark: { background: '#151b25', foreground: '#f6f8fa', cursor: '#ffffff', selectionBackground: '#6e778a' },
};

export const useContainerExecTerminal = (containerId?: string, disabled?: boolean, target?: ExecTarget) => {
  const { theme } = useLayoutContext();
  const [shell, setShell] = useState<'bash' | 'sh'>('bash');
  const [isActive, setIsActive] = useState(false);

  const sessionId = useMemo(() => nanoid(), []);
  const groupId = containerId ? `container-exec:${containerId}:${sessionId}` : undefined;

  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef(new FitAddon());
  const containerRef = useRef<HTMLDivElement | null>(null);
  const hubRef = useRef<HubConnection | null>(null);

  const execStartedRef = useRef(false);

  const handleExecOutput = useCallback((data: Uint8Array | ArrayBuffer) => {
    termRef.current?.write(new Uint8Array(data));
  }, []);

  useEffect(() => {
    if (disabled || !containerRef.current || !containerId) return;

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: 'monospace',
      fontSize: 13,
      theme: theme.mode === 'dark' ? THEMES.dark : THEMES.light,
    });

    term.loadAddon(fitRef.current);
    term.open(containerRef.current);
    fitRef.current.fit();
    termRef.current = term;

    const onDataDisposable = term.onData((data) => {
      if (execStartedRef.current && hubRef.current?.state === HubConnectionState.Connected && groupId) {
        const bytes = new TextEncoder().encode(data);
        hubRef.current.invoke('SendExecInput', groupId, bytes, target ?? 'Container').catch(console.error);
      }
    });

    const ro = new ResizeObserver(() => {
      fitRef.current.fit();

      if (
        execStartedRef.current &&
        hubRef.current?.state === HubConnectionState.Connected &&
        groupId &&
        termRef.current
      ) {
        hubRef.current.invoke('ResizeExec', groupId, termRef.current.cols, termRef.current.rows, target ?? 'Container').catch(console.error);
      }
    });

    ro.observe(containerRef.current);

    return () => {
      ro.disconnect();
      onDataDisposable.dispose();
      term.dispose();
      termRef.current = null;
    };
  }, [containerId, disabled, groupId, theme.mode, target]);

  useEffect(() => {
    if (!disabled) return;
    termRef.current?.dispose();
    termRef.current = null;
    hubRef.current = null;
    execStartedRef.current = false;
  }, [disabled]);

  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.theme = theme.mode === 'dark' ? THEMES.dark : THEMES.light;
    }
  }, [theme.mode]);

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hubRef.current = hub;
      hub.on('SendContainerExec', handleExecOutput);
    },
    [handleExecOutput],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off('SendContainerExec', handleExecOutput);
      hubRef.current = null;
    },
    [handleExecOutput],
  );

  const onJoinedGroup = useCallback(
    async (hub: HubConnection) => {
      if (!groupId) return;

      try {
        await hub.invoke('StartExecProcess', groupId, shell, target ?? 'Container');
        execStartedRef.current = true;

        if (termRef.current) {
          await hub.invoke('ResizeExec', groupId, termRef.current.cols, termRef.current.rows, target ?? 'Container');
        }
      } catch {
        termRef.current?.writeln('\r\n\x1b[31m[failed to start process]\x1b[0m');
      }
    },
    [groupId, shell],
  );

  const { isLoading, isConnected } = useSignalRGroup({
    groupName: groupId,
    enabled: isActive && !disabled,
    skip: !groupId || disabled,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
  });

  const toggleConnection = () => {
    if (isConnected) {
      setIsActive(false);
      execStartedRef.current = false;
      termRef.current?.writeln('\r\n\x1b[31m[disconnected]\x1b[0m');
    } else {
      execStartedRef.current = false;
      termRef.current?.clear();
      termRef.current?.focus();
      setIsActive(true);
    }
  };

  return {
    terminalRef: containerRef,
    isLoading,
    isConnected,
    shell,
    setShell,
    toggleConnection,
  };
};
