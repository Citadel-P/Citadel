import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
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

type Shell = 'bash' | 'sh';

interface ShellOption {
  label: string;
  value: Shell;
}

interface ExecTerminalProps {
  terminalRef: React.RefObject<HTMLDivElement | null>;
  shell: Shell;
  setShell: (shell: Shell) => void;
  isLoading: boolean;
  isConnected: boolean;
  disabled?: boolean;
  toolbarStart?: ReactNode;
  toggleConnection: () => void;
}

interface UseContainerExecOptions {
  containerId?: string;
  disabled?: boolean;
  deploymentId?: string;
  stackId?: string;
  methodNames?: {
    sendExecInput?: string;
    resizeExec?: string;
    startExecProcess?: string;
  };
}

const SHELLS: ShellOption[] = [
  { label: 'bash', value: 'bash' },
  { label: 'sh', value: 'sh' },
];

const THEMES = {
  light: {
    background: '#f7f8f9',
    foreground: '#24292e',
    cursor: '#24292e',
    selectionBackground: '#c8d9fa',
  },
  dark: {
    background: '#151b25',
    foreground: '#f6f8fa',
    cursor: '#ffffff',
    selectionBackground: '#6e778a',
  },
} as const;

const ExecTerminal: React.FC<ExecTerminalProps> = ({
  terminalRef,
  shell,
  setShell,
  isLoading,
  isConnected,
  disabled,
  toolbarStart,
  toggleConnection,
}) => {
  return (
    <div className="flex flex-col w-full h-[60vh]">
      <div className="flex items-center justify-between bg-secondary/20 p-2 rounded-t-md">
        <div className="flex flex-row gap-4">
          {toolbarStart}
          <Select value={shell} disabled={isConnected || isLoading} onValueChange={(value) => setShell(value as Shell)}>
            <SelectTrigger className="w-32 max-h-8 bg-background rounded-sm shadow-xs">
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
            disabled={disabled || isLoading}
            variant="outline"
            className="rounded-sm h-8 shadow-xs text-sm font-normal"
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

export const useContainerExecTerminal = (options: UseContainerExecOptions) => {
  const { containerId, disabled = false, deploymentId, stackId, methodNames = {} } = options;

  const {
    sendExecInput = 'SendExecInput',
    resizeExec = 'ResizeExec',
    startExecProcess = 'StartExecProcess',
  } = methodNames;

  const { theme } = useLayoutContext();
  const [shell, setShell] = useState<Shell>('bash');
  const [isActive, setIsActive] = useState(false);

  const sessionId = useMemo(() => nanoid(), []);
  const groupId = containerId ? `container-exec:${containerId}:${sessionId}` : undefined;
  const targetIds = useMemo(() => {
    if (stackId && containerId) return [stackId, containerId];
    return [deploymentId ?? containerId];
  }, [containerId, deploymentId, stackId]);

  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const hubRef = useRef<HubConnection | null>(null);
  const execStartedRef = useRef(false);

  useEffect(() => {
    if (disabled || !containerRef.current || !containerId) return;

    const fitAddon = new FitAddon();
    fitAddonRef.current = fitAddon;

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: 'monospace',
      fontSize: 13,
      theme: theme.mode === 'dark' ? THEMES.dark : THEMES.light,
    });

    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();
    termRef.current = term;

    const dataDisposable = term.onData((data) => {
      if (!execStartedRef.current) return;
      if (hubRef.current?.state !== HubConnectionState.Connected) return;
      if (!groupId || targetIds.some((targetId) => !targetId)) return;

      const bytes = new TextEncoder().encode(data);
      hubRef.current.invoke(sendExecInput, ...targetIds, sessionId, bytes).catch(console.error);
    });

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();

      if (!execStartedRef.current) return;
      if (hubRef.current?.state !== HubConnectionState.Connected) return;
      if (!groupId || targetIds.some((targetId) => !targetId) || !termRef.current) return;

      hubRef.current
        .invoke(resizeExec, ...targetIds, sessionId, termRef.current.cols, termRef.current.rows)
        .catch(console.error);
    });

    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      dataDisposable.dispose();
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [containerId, disabled, groupId, sessionId, targetIds, sendExecInput, resizeExec, theme.mode]);

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

  const handleExecOutput = useCallback((data: Uint8Array | ArrayBuffer) => {
    termRef.current?.write(new Uint8Array(data));
  }, []);

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
      if (!groupId || targetIds.some((targetId) => !targetId)) return;

      try {
        await hub.invoke(startExecProcess, ...targetIds, sessionId, shell);
        execStartedRef.current = true;

        if (termRef.current) {
          await hub.invoke(resizeExec, ...targetIds, sessionId, termRef.current.cols, termRef.current.rows);
        }
      } catch {
        termRef.current?.writeln('\r\n\x1b[31m[failed to start process]\x1b[0m');
      }
    },
    [groupId, targetIds, sessionId, shell, startExecProcess, resizeExec],
  );

  const { isLoading, isConnected } = useSignalRGroup({
    groupName: groupId,
    enabled: isActive && !disabled,
    skip: !groupId || disabled,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
  });

  const toggleConnection = useCallback(() => {
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
  }, [isConnected]);

  return {
    terminalRef: containerRef,
    isLoading,
    isConnected,
    shell,
    setShell,
    toggleConnection,
  };
};

export const DeploymentExec: React.FC<{
  deploymentId: string;
  containerId: string | undefined;
  disabled?: boolean;
}> = ({ deploymentId, containerId, disabled }) => {
  const nid = normalizeDockerId(containerId);
  const terminal = useContainerExecTerminal({
    containerId: nid,
    disabled,
    deploymentId,
    methodNames: {
      sendExecInput: 'SendDeploymentExecInput',
      resizeExec: 'ResizeDeploymentExec',
      startExecProcess: 'StartDeploymentExecProcess',
    },
  });

  return <ExecTerminal {...terminal} disabled={disabled} />;
};

export const ContainerExec: React.FC<{
  containerId?: string;
  disabled?: boolean;
}> = ({ containerId, disabled }) => {
  const nid = normalizeDockerId(containerId);
  const terminal = useContainerExecTerminal({ containerId: nid, disabled });

  return <ExecTerminal {...terminal} disabled={disabled} />;
};

export const StackExec: React.FC<{
  stackId: string;
  containerId?: string;
  disabled?: boolean;
  toolbarStart?: ReactNode;
}> = ({ stackId, containerId, disabled, toolbarStart }) => {
  const nid = normalizeDockerId(containerId);
  const terminal = useContainerExecTerminal({
    containerId: nid,
    disabled,
    stackId,
    methodNames: {
      sendExecInput: 'SendStackExecInput',
      resizeExec: 'ResizeStackExec',
      startExecProcess: 'StartStackExecProcess',
    },
  });

  return <ExecTerminal {...terminal} disabled={disabled} toolbarStart={toolbarStart} />;
};
