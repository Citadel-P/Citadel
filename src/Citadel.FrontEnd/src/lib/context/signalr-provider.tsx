import {
  HubConnection,
  HubConnectionBuilder,
  HttpTransportType,
  IHttpConnectionOptions,
  HubConnectionState,
} from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/auth-context';
import { SignalRContext } from './signalr-context';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { MessagePackHubProtocol } from '@microsoft/signalr-protocol-msgpack';

export const SignalRProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  type GroupState = { state: 'joining' | 'joined'; references: number };
  const [connectionState, setConnectionState] = useState<HubConnectionState>(HubConnectionState.Disconnected);
  const [connection, setConnection] = useState<HubConnection | null>(null);
  const { accessToken } = useAuthContext();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const tokenRef = useRef<string | undefined>(accessToken);
  const prevTokenRef = useRef<string | undefined>(accessToken);
  const isCanceledRef = useRef(false);
  const readyResolveRef = useRef<(() => void) | null>(null);
  const readyPromiseRef = useRef<Promise<void> | null>(null);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  const buildConnection = useCallback(() => {
    isCanceledRef.current = false;

    const conn = new HubConnectionBuilder()
      .withUrl(`${baseUrl}/hubs/global`, {
        accessTokenFactory: () => tokenRef.current ?? '',
        transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
      } as IHttpConnectionOptions)
      .withAutomaticReconnect()
      .withHubProtocol(new MessagePackHubProtocol())
      .build();

    conn.onreconnecting(() => setConnectionState(HubConnectionState.Reconnecting));
    conn.onreconnected(async () => {
      setConnectionState(HubConnectionState.Connected);
      // Rejoin groups on reconnect
      for (const [groupName, group] of groupStates.current.entries()) {
        if (group.state === 'joined' || group.state === 'joining') {
          try {
            await conn.send('JoinGroup', groupName);
            groupStates.current.set(groupName, { ...group, state: 'joined' });
          } catch (err) {
            console.error(`Failed to rejoin group ${groupName}:`, err);
          }
        }
      }
    });
    conn.onclose(() => {
      setConnectionState(HubConnectionState.Disconnected);
    });

    readyPromiseRef.current = null;

    (async () => {
      try {
        await startConnectionWithRetry(conn, isCanceledRef);
        if (isCanceledRef.current) {
          await conn.stop().catch(() => {});
          setConnection(null);
          return;
        }
        setConnection(conn);
        setConnectionState(conn.state);
        readyResolveRef.current?.();
      } catch (err) {
        console.error('[SignalR] final connection failure', err);
        await conn.stop().catch(() => {});
        setConnectionState(conn.state ?? HubConnectionState.Disconnected);
        setConnection(null);
      }
    })();

    return conn;
  }, [baseUrl]);

  const rebuildConnection = useCallback(() => {
    isCanceledRef.current = true;
    connection
      ?.stop()
      .catch(console.error)
      .finally(() => {
        setConnection(null);
        isCanceledRef.current = false;
        buildConnection();
      });
  }, [connection, buildConnection]);

  // Sync token ref (always OK to do in effect)
  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  // Rebuild connection only when the token actually changes
  useEffect(() => {
    if (accessToken === prevTokenRef.current) return;
    prevTokenRef.current = accessToken;

    rebuildConnection();
  }, [accessToken, rebuildConnection]);

  const ensureReadyPromise = () => {
    if (!readyPromiseRef.current) {
      readyPromiseRef.current = new Promise<void>((resolve) => {
        readyResolveRef.current = resolve;
      });
    }
    return readyPromiseRef.current;
  };

  // Initial connect on mount
  useEffect(() => {
    buildConnection();
    return () => {
      isCanceledRef.current = true;
      connection?.stop().catch(console.error);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // OK to omit buildConnection here – only run once

  const ensureConnectionReady = useCallback(async () => {
    if (connection && connection.state === HubConnectionState.Connected) return connection;
    await ensureReadyPromise();
    if (connection && connection.state === HubConnectionState.Connected) return connection;
    if (connection) return connection;
    throw new Error('SignalR connection not available');
  }, [connection]);

  const joinGroup = useCallback(
    async (groupName: string, setup?: (hub: HubConnection) => void) => {
      if (!groupName) return;

      await ensureConnectionReady();
      const conn = connection!;

      try {
        setup?.(conn);
      } catch (err) {
        console.error('setup callback threw:', err);
      }

      const group = groupStates.current.get(groupName);
      if (group) {
        groupStates.current.set(groupName, { ...group, references: group.references + 1 });
        return;
      }

      groupStates.current.set(groupName, { state: 'joining', references: 1 });

      try {
        await conn.send('JoinGroup', groupName);
        const current = groupStates.current.get(groupName);
        if (current) {
          groupStates.current.set(groupName, { ...current, state: 'joined' });
        }
      } catch (err) {
        console.error('JoinGroup failed:', err);
        groupStates.current.delete(groupName);
      }
    },
    [connection, ensureConnectionReady],
  );

  const leaveGroup = useCallback(
    async (groupName: string, remove?: (hub: HubConnection) => void) => {
      if (!groupName) return;

      if (connection) {
        try {
          remove?.(connection);
        } catch (err) {
          console.error('remove callback threw:', err);
        }
      }

      const group = groupStates.current.get(groupName);
      if (!group) return;

      if (group.references > 1) {
        groupStates.current.set(groupName, { ...group, references: group.references - 1 });
        return;
      }

      groupStates.current.delete(groupName);

      if (connection?.state === HubConnectionState.Connected) {
        try {
          await connection.send('LeaveGroup', groupName);
        } catch (err) {
          console.warn('LeaveGroup failed:', err);
        }
      }
    },
    [connection],
  );

  return (
    <SignalRContext.Provider value={{ connection, connectionState, joinGroup, leaveGroup }}>
      {children}
    </SignalRContext.Provider>
  );
};
