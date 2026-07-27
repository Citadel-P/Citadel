import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/auth-context';
import { SignalRContext } from './signalr-context';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { createSignalRConnection, SignalRConnectionFactory } from '../createSignalRConnection';
import { useQueryClient } from '@tanstack/react-query';

type StartConnection = typeof startConnectionWithRetry;

type SignalRProviderProps = {
  children?: React.ReactNode;
  connectionFactory?: SignalRConnectionFactory;
  startConnection?: StartConnection;
};

type GroupState = {
  state: 'pending' | 'joining' | 'joined';
  references: number;
  joinPromise?: Promise<void>;
};

type CancellationRef = { current: boolean };

const streamedStatsQueryKeys = new Set([
  'getContainerStats',
  'getDeploymentStats',
  'getPlatformStats',
  'getStackStats',
]);

export const SignalRProvider: React.FC<SignalRProviderProps> = ({
  children,
  connectionFactory = createSignalRConnection,
  startConnection = startConnectionWithRetry,
}) => {
  const [connectionState, setConnectionState] = useState<HubConnectionState>(HubConnectionState.Disconnected);
  const [connection, setConnection] = useState<HubConnection | null>(null);
  const { accessToken } = useAuthContext();
  const queryClient = useQueryClient();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const tokenRef = useRef<string | undefined>(accessToken);
  const prevTokenRef = useRef<string | undefined>(accessToken);
  const activeCancelRef = useRef<CancellationRef | null>(null);
  const activeConnectionRef = useRef<HubConnection | null>(null);
  const readyPromiseRef = useRef<Promise<HubConnection> | null>(null);
  const rebuildGenerationRef = useRef(0);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  const buildConnection = useCallback(() => {
    if (activeCancelRef.current) {
      activeCancelRef.current.current = true;
    }

    const cancelRef: CancellationRef = { current: false };
    activeCancelRef.current = cancelRef;

    const conn = connectionFactory({
      baseUrl,
      accessTokenFactory: () => tokenRef.current ?? '',
    });

    activeConnectionRef.current = conn;
    conn.on('LicenseStateChanged', () => {
      void queryClient.invalidateQueries({ queryKey: ['getLicenseEntitlements'] });
      void queryClient.invalidateQueries({ queryKey: ['getLicense'] });
    });

    conn.onreconnecting(() => {
      if (activeConnectionRef.current === conn) {
        setConnectionState(HubConnectionState.Reconnecting);
      }
    });

    const rejoinGroup = async (groupName: string) => {
      const group = groupStates.current.get(groupName);
      if (!group || activeConnectionRef.current !== conn || conn.state !== HubConnectionState.Connected) {
        return;
      }

      const joinPromise = conn.invoke('JoinGroup', groupName);
      groupStates.current.set(groupName, { ...group, state: 'joining', joinPromise });

      try {
        await joinPromise;

        const current = groupStates.current.get(groupName);
        if (current?.joinPromise === joinPromise) {
          groupStates.current.set(groupName, {
            ...current,
            state: 'joined',
            joinPromise: undefined,
          });
        }
      } catch (err) {
        const current = groupStates.current.get(groupName);
        if (current?.joinPromise === joinPromise) {
          groupStates.current.set(groupName, {
            ...current,
            state: 'pending',
            joinPromise: undefined,
          });

          setTimeout(() => {
            const pendingGroup = groupStates.current.get(groupName);
            if (
              activeConnectionRef.current === conn &&
              conn.state === HubConnectionState.Connected &&
              pendingGroup?.state === 'pending'
            ) {
              void rejoinGroup(groupName);
            }
          }, 2_000);
        }

        console.error(`Failed to rejoin group ${groupName}:`, err);
      }
    };

    conn.onreconnected(async () => {
      if (activeConnectionRef.current !== conn) {
        return;
      }

      setConnectionState(HubConnectionState.Connected);

      for (const groupName of groupStates.current.keys()) {
        await rejoinGroup(groupName);
      }

      await queryClient.invalidateQueries({
        predicate: (query) => streamedStatsQueryKeys.has(String(query.queryKey[0])),
        refetchType: 'active',
      });
    });

    conn.onclose(() => {
      if (activeConnectionRef.current === conn) {
        setConnectionState(HubConnectionState.Disconnected);
      }
    });

    const readyPromise = (async () => {
      try {
        await startConnection(conn, cancelRef);

        if (cancelRef.current || activeConnectionRef.current !== conn) {
          await conn.stop().catch(() => {});
          throw new Error('SignalR connection was replaced before it became ready');
        }

        if (conn.state !== HubConnectionState.Connected) {
          throw new Error('SignalR connection did not reach the connected state');
        }

        setConnection(conn);
        setConnectionState(conn.state);
        return conn;
      } catch (err) {
        if (!cancelRef.current && activeConnectionRef.current === conn) {
          console.error('[SignalR] final connection failure', err);
        }

        await conn.stop().catch(() => {});
        if (activeConnectionRef.current === conn) {
          setConnectionState(conn.state ?? HubConnectionState.Disconnected);
          setConnection(null);
        }
        throw err;
      }
    })();

    readyPromiseRef.current = readyPromise;
    void readyPromise.catch(() => {});

    return conn;
  }, [baseUrl, connectionFactory, queryClient, startConnection]);

  const rebuildConnection = useCallback(() => {
    if (activeCancelRef.current) {
      activeCancelRef.current.current = true;
    }

    const rebuildGeneration = ++rebuildGenerationRef.current;
    const currentConnection = activeConnectionRef.current;
    if (!currentConnection) {
      setConnection(null);
      buildConnection();
      return;
    }

    currentConnection
      .stop()
      .catch(console.error)
      .finally(() => {
        if (rebuildGenerationRef.current !== rebuildGeneration) {
          return;
        }

        setConnection(null);
        buildConnection();
      });
  }, [buildConnection]);

  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  useEffect(() => {
    if (accessToken === prevTokenRef.current) {
      return;
    }

    prevTokenRef.current = accessToken;
    rebuildConnection();
  }, [accessToken, rebuildConnection]);

  useEffect(() => {
    buildConnection();
    return () => {
      rebuildGenerationRef.current += 1;
      if (activeCancelRef.current) {
        activeCancelRef.current.current = true;
      }
      const activeConnection = activeConnectionRef.current;
      activeConnectionRef.current = null;
      readyPromiseRef.current = null;
      activeConnection?.stop().catch(console.error);
    };
    // The connection is rebuilt explicitly when the access token changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const ensureConnectionReady = useCallback(async () => {
    const activeConnection = activeConnectionRef.current;
    if (activeConnection?.state === HubConnectionState.Connected) {
      return activeConnection;
    }

    const readyConnection = await readyPromiseRef.current;
    if (readyConnection?.state === HubConnectionState.Connected) {
      return readyConnection;
    }

    throw new Error('SignalR connection not available');
  }, []);

  const joinGroup = useCallback(
    async (groupName: string, setup?: (hub: HubConnection) => void) => {
      if (!groupName) {
        return;
      }

      const conn = await ensureConnectionReady();

      try {
        setup?.(conn);
      } catch (err) {
        console.error('setup callback threw:', err);
      }

      const group = groupStates.current.get(groupName);
      if (group) {
        groupStates.current.set(groupName, { ...group, references: group.references + 1 });
        await group.joinPromise;
        return;
      }

      const joinPromise = conn.invoke('JoinGroup', groupName);
      groupStates.current.set(groupName, { state: 'joining', references: 1, joinPromise });

      try {
        await joinPromise;
        const current = groupStates.current.get(groupName);
        if (current) {
          groupStates.current.set(groupName, {
            ...current,
            state: 'joined',
            joinPromise: undefined,
          });
        }
      } catch (err) {
        console.error('JoinGroup failed:', err);
        groupStates.current.delete(groupName);
        throw err;
      }
    },
    [ensureConnectionReady],
  );

  const leaveGroup = useCallback(
    async (groupName: string, remove?: (hub: HubConnection) => void) => {
      if (!groupName) {
        return;
      }

      if (connection) {
        try {
          remove?.(connection);
        } catch (err) {
          console.error('remove callback threw:', err);
        }
      }

      const group = groupStates.current.get(groupName);
      if (!group) {
        return;
      }

      if (group.references > 1) {
        groupStates.current.set(groupName, { ...group, references: group.references - 1 });
        return;
      }

      groupStates.current.delete(groupName);

      if (connection?.state === HubConnectionState.Connected) {
        try {
          await connection.invoke('LeaveGroup', groupName);
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
