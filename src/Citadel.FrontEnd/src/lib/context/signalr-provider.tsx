import {
  HubConnection,
  HubConnectionBuilder,
  HttpTransportType,
  IHttpConnectionOptions,
  HubConnectionState,
} from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/AuthContext';
import { SignalRContext } from './signalr-context';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { MessagePackHubProtocol } from '@microsoft/signalr-protocol-msgpack';

export const SignalRProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  type GroupState = 'joining' | 'joined';
  const [connectionState, setConnectionState] = useState<HubConnectionState>(HubConnectionState.Disconnected);
  const [connection, setConnection] = useState<HubConnection | null>(null);
  const { accessToken } = useAuthContext();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const tokenRef = useRef<string | undefined>(accessToken);
  const isCanceledRef = useRef(false);
  const readyResolveRef = useRef<() => void | null>(null);
  const readyPromiseRef = useRef<Promise<void> | null>(null);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  // ensure accessTokenFactory reads latest token
  useEffect(() => {
    tokenRef.current = accessToken;
    // if token changed while we have a running connection, rebuild it so the server will accept it
    // (only if you require new token to be active immediately)
    // you can also decide to not rebuild here and rely on token expiry + reconnect
    // For now, rebuild to be safe:
    rebuildConnection();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accessToken]);

  const ensureReadyPromise = () => {
    if (!readyPromiseRef.current) {
      readyPromiseRef.current = new Promise<void>((resolve) => {
        readyResolveRef.current = resolve;
      });
    }
    return readyPromiseRef.current;
  };

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
      for (const [groupName, state] of groupStates.current.entries()) {
        if (state === 'joined' || state === 'joining') {
          try {
            await conn.send('JoinGroup', groupName);
            groupStates.current.set(groupName, 'joined');
          } catch (err) {
            console.error(`Failed to rejoin group ${groupName}:`, err);
          }
        }
      }
    });
    conn.onclose(() => {
      setConnectionState(HubConnectionState.Disconnected);
      // Optional: clear activeGroups.current here if groups should reset on disconnect
    });

    readyPromiseRef.current = null;
    ensureReadyPromise();

    (async () => {
      try {
        await startConnectionWithRetry(conn, isCanceledRef);
        if (isCanceledRef.current) {
          await conn.stop().catch(() => {});
          setConnection(null); // explicit cleanup on cancel
          return;
        }
        setConnection(conn);
        setConnectionState(conn.state);
        readyResolveRef.current?.();
      } catch (err) {
        console.error('[SignalR] final connection failure', err);
        await conn.stop().catch(() => {});
        setConnectionState(conn.state ?? HubConnectionState.Disconnected);
        setConnection(null); // ensure cleanup on failure
      }
    })();

    return conn;
  }, [baseUrl]);

  // start once on mount
  useEffect(() => {
    buildConnection();
    return () => {
      isCanceledRef.current = true;
      // stop existing connection
      connection?.stop().catch(console.error);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // run once

  // rebuild helper
  const rebuildConnection = useCallback(() => {
    isCanceledRef.current = true;
    // stop current connection first
    connection
      ?.stop()
      .catch(console.error)
      .finally(() => {
        setConnection(null);
        // reset cancel flag and start new connection
        isCanceledRef.current = false;
        buildConnection();
      });
  }, [connection, buildConnection]);

  // wait until connection is ready (connected)
  const ensureConnectionReady = useCallback(async () => {
    if (connection && connection.state === HubConnectionState.Connected) return connection;
    // wait for the readyPromise
    await ensureReadyPromise();
    // after it resolves, set connection may have changed
    if (connection && connection.state === HubConnectionState.Connected) return connection;
    // otherwise try to return whatever is in state
    if (connection) return connection;
    throw new Error('SignalR connection not available');
  }, [connection]);

  // joinGroup: registers handlers (setup) before sending JoinGroup to avoid race
  const joinGroup = useCallback(
    async (groupName: string, setup?: (hub: HubConnection) => void) => {
      if (!groupName) return;

      const state = groupStates.current.get(groupName);
      if (state === 'joining' || state === 'joined') return; // already in process or done

      groupStates.current.set(groupName, 'joining');

      await ensureConnectionReady();
      const conn = connection!;

      try {
        setup?.(conn);
      } catch (err) {
        console.error('setup callback threw:', err);
      }

      try {
        await conn.send('JoinGroup', groupName);
        groupStates.current.set(groupName, 'joined');
      } catch (err) {
        console.error('JoinGroup failed:', err);
        groupStates.current.delete(groupName); // allow retry
      }
    },
    [connection, ensureConnectionReady],
  );

  const leaveGroup = useCallback(
    async (groupName: string, remove?: (hub: HubConnection) => void) => {
      if (!groupName) return;

      const state = groupStates.current.get(groupName);
      if (!state) return; // nothing to leave

      groupStates.current.delete(groupName);

      if (connection?.state === HubConnectionState.Connected) {
        try {
          await connection.send('LeaveGroup', groupName);
        } catch (err) {
          console.warn('LeaveGroup failed:', err);
        }
      }

      if (connection) {
        try {
          remove?.(connection);
        } catch (err) {
          console.error('remove callback threw:', err);
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
