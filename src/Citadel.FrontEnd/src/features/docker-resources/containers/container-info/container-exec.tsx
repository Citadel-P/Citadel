import { useCallback, useEffect, useRef } from 'react';
import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { normalizeDockerId } from '@/lib/utils';
import { ITheme } from '@xterm/xterm';
import { useLayoutContext } from '@/lib/context/layout-context';

export const ContainerExec = ({ containerId }: { containerId?: string }) => {
  const nid = normalizeDockerId(containerId);
  const { terminalRef } = useContainerExecTerminal(nid);

  return <div ref={terminalRef} className="w-full h-[55vh]" />;
};

const LIGHT_THEME: ITheme = {
  background: '#f7f8f9',
  foreground: '#24292e',
  cursor: '#24292e',
  selectionBackground: '#c8d9fa',
};

const DARK_THEME: ITheme = {
  background: '#151b25',
  foreground: '#f6f8fa',
  cursor: '#ffffff',
  selectionBackground: '#6e778a',
};

const useContainerExecTerminal = (containerId?: string) => {
  const { theme } = useLayoutContext();
  const currentTheme = theme.mode === 'dark' ? DARK_THEME : LIGHT_THEME;
  const groupId = `container-exec:${containerId}`;

  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef(new FitAddon());
  const containerRef = useRef<HTMLDivElement | null>(null);
  const hubRef = useRef<HubConnection | null>(null);

  const refreshTerminal = useCallback(() => {
    if (!termRef.current || !containerRef.current) return;

    const term = termRef.current;
    if (containerRef.current.clientWidth > 0) {
      fitRef.current.fit();

      if (hubRef.current?.state === 'Connected') {
        hubRef.current
          .invoke('ResizeExec', groupId, term.cols, term.rows)
          .catch((e) => console.debug('Resize failed', e));
      }
    }
  }, [groupId]);

  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.theme = currentTheme;
    }
  }, [currentTheme]);

  const { isLoading } = useSignalRGroup({
    groupName: groupId,
    setupEventListeners: (hub) => {
      hubRef.current = hub;
      hub.on('SendContainerExec', (data) => termRef.current?.write(new Uint8Array(data)));
      setTimeout(refreshTerminal, 100);
    },
    removeEventListeners: (hub) => hub.off('SendContainerExec'),
    skip: !containerId,
  });

  useEffect(() => {
    if (!containerRef.current || !containerId || termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      cursorStyle: 'block',
      fontFamily: 'monospace',
      scrollback: 5000,
      theme: currentTheme,
      fontSize: 13,
    });

    term.loadAddon(fitRef.current);
    term.open(containerRef.current);
    term.focus();

    document.fonts.ready.then(() => {
      requestAnimationFrame(() => {
        refreshTerminal();
      });
    });

    term.onData((data) => {
      if (hubRef.current?.state === HubConnectionState.Connected) {
        hubRef.current.invoke('SendExecInput', groupId, new TextEncoder().encode(data));
      }
    });

    const ro = new ResizeObserver(() => refreshTerminal());
    ro.observe(containerRef.current);

    termRef.current = term;
    return () => {
      ro.disconnect();
      term.dispose();
      termRef.current = null;
    };
  }, [containerId, groupId, refreshTerminal]);

  useEffect(() => {
    if (hubRef.current?.state === HubConnectionState.Disconnected) {
      termRef.current?.writeln('\r\n\x1b[33m[connection closed]\x1b[0m');
    }
  }, [hubRef.current?.state]);

  return { terminalRef: containerRef, isLoading };
};
