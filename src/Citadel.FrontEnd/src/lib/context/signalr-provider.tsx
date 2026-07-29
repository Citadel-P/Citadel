import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/auth-context';
import { createSignalRConnection, SignalRConnectionFactory } from '../createSignalRConnection';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { LiveConnectionState, SignalRContext } from './signalr-context';

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

const liveBackedQueryKeys = new Set([
  'getAutomationAction',
  'getBackupPolicy',
  'getBackupRepository',
  'getBuildAgentPool',
  'getBuildProject',
  'getBuildRun',
  'getBuildRunLogs',
  'getContainerData',
  'getContainersData',
  'getContainerStats',
  'getDeployment',
  'getDeploymentStats',
  'getGitRepository',
  'getPlatformStats',
  'getPlatfom',
  'getStack',
  'getStackStats',
  'listActivities',
  'listAlertEvents',
  'listAutomationActions',
  'listBackupPolicies',
  'listBackupRepositories',
  'listBackupRestoreRuns',
  'listBackupRuns',
  'listBuildAgentPools',
  'listBuildProjects',
  'listBuildRuns',
  'listContainers',
  'listDeployments',
  'listGitRepositories',
  'listImages',
  'listPlatforms',
  'listStacks',
]);

const isBrowserOffline = () => typeof navigator !== 'undefined' && navigator.onLine === false;

export const SignalRProvider: React.FC<SignalRProviderProps> = ({
  children,
  connectionFactory = createSignalRConnection,
  startConnection = startConnectionWithRetry,
}) => {
  const [connectionState, setConnectionState] = useState<HubConnectionState>(HubConnectionState.Disconnected);
  const [liveConnectionState, setLiveConnectionState] = useState<LiveConnectionState>('connecting');
  const [interruptedAt, setInterruptedAt] = useState<number>();
  const [lastConnectedAt, setLastConnectedAt] = useState<number>();
  const [connection, setConnection] = useState<HubConnection | null>(null);
  const { accessToken } = useAuthContext();
  const queryClient = useQueryClient();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;

  const tokenRef = useRef<string | undefined>(accessToken);
  const prevTokenRef = useRef<string | undefined>(accessToken);
  const activeCancelRef = useRef<CancellationRef | null>(null);
  const activeConnectionRef = useRef<HubConnection | null>(null);
  const readyPromiseRef = useRef<Promise<HubConnection> | null>(null);
  const rebuildPromiseRef = useRef<Promise<HubConnection> | null>(null);
  const retryPromiseRef = useRef<Promise<void> | null>(null);
  const startInProgressRef = useRef(false);
  const rebuildGenerationRef = useRef(0);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  const markConnected = useCallback(() => {
    setConnectionState(HubConnectionState.Connected);
    setLiveConnectionState('connected');
    setInterruptedAt(undefined);
    setLastConnectedAt(Date.now());
  }, []);

  const markInterrupted = useCallback(
    (state: Extract<LiveConnectionState, 'reconnecting' | 'disconnected' | 'offline'>) => {
      setLiveConnectionState(state);
      setInterruptedAt((current) => current ?? Date.now());
    },
    [],
  );

  const reconcileLiveQueries = useCallback(
    () =>
      queryClient.invalidateQueries({
        predicate: (query) => liveBackedQueryKeys.has(String(query.queryKey[0])),
        refetchType: 'active',
      }),
    [queryClient],
  );

  const buildConnection = useCallback(
    (reconcileAfterConnect = false) => {
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
      startInProgressRef.current = true;
      setConnectionState(HubConnectionState.Connecting);
      if (isBrowserOffline()) {
        markInterrupted('offline');
      } else {
        setLiveConnectionState('connecting');
      }

      conn.on('LicenseStateChanged', () => {
        void queryClient.invalidateQueries({ queryKey: ['getLicenseEntitlements'] });
        void queryClient.invalidateQueries({ queryKey: ['getLicense'] });
      });

      conn.onreconnecting(() => {
        if (activeConnectionRef.current !== conn) {
          return;
        }

        setConnectionState(HubConnectionState.Reconnecting);
        markInterrupted(isBrowserOffline() ? 'offline' : 'reconnecting');
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

        setConnection(conn);
        markConnected();

        for (const groupName of groupStates.current.keys()) {
          await rejoinGroup(groupName);
        }

        await reconcileLiveQueries();
      });

      conn.onclose(() => {
        if (activeConnectionRef.current !== conn) {
          return;
        }

        setConnectionState(HubConnectionState.Disconnected);
        markInterrupted(isBrowserOffline() ? 'offline' : 'disconnected');
      });

      const readyPromise = (async () => {
        try {
          await startConnection(conn, cancelRef, {
            onRetryAttempt: () => {
              if (!cancelRef.current && activeConnectionRef.current === conn) {
                markInterrupted(isBrowserOffline() ? 'offline' : 'reconnecting');
              }
            },
          });

          if (cancelRef.current || activeConnectionRef.current !== conn) {
            await conn.stop().catch(() => {});
            throw new Error('SignalR connection was replaced before it became ready');
          }

          if (conn.state !== HubConnectionState.Connected) {
            throw new Error('SignalR connection did not reach the connected state');
          }

          setConnection(conn);
          markConnected();

          if (reconcileAfterConnect) {
            await reconcileLiveQueries();
          }

          return conn;
        } catch (err) {
          if (!cancelRef.current && activeConnectionRef.current === conn) {
            console.error('[SignalR] final connection failure', err);
          }

          await conn.stop().catch(() => {});
          if (activeConnectionRef.current === conn) {
            setConnectionState(HubConnectionState.Disconnected);
            setConnection(null);
            markInterrupted(isBrowserOffline() ? 'offline' : 'disconnected');
          }
          throw err;
        } finally {
          if (activeConnectionRef.current === conn) {
            startInProgressRef.current = false;
          }
        }
      })();

      readyPromiseRef.current = readyPromise;
      void readyPromise.catch(() => {});
      return readyPromise;
    },
    [baseUrl, connectionFactory, markConnected, markInterrupted, queryClient, reconcileLiveQueries, startConnection],
  );

  const rebuildConnection = useCallback(
    (reconcileAfterConnect = false) => {
      if (rebuildPromiseRef.current) {
        return rebuildPromiseRef.current;
      }

      if (activeCancelRef.current) {
        activeCancelRef.current.current = true;
      }

      const rebuildGeneration = ++rebuildGenerationRef.current;
      const currentConnection = activeConnectionRef.current;

      activeConnectionRef.current = null;
      readyPromiseRef.current = null;
      startInProgressRef.current = false;
      setConnection(null);
      setConnectionState(HubConnectionState.Connecting);
      if (isBrowserOffline()) {
        markInterrupted('offline');
      } else {
        setLiveConnectionState('connecting');
      }

      const rebuildPromise = (async () => {
        if (currentConnection) {
          await currentConnection.stop().catch((err) => {
            console.warn('[SignalR] failed to stop replaced connection', err);
          });
        }

        if (rebuildGenerationRef.current !== rebuildGeneration) {
          throw new Error('SignalR connection rebuild was canceled');
        }

        return buildConnection(reconcileAfterConnect);
      })();

      rebuildPromiseRef.current = rebuildPromise;
      const clearRebuild = () => {
        if (rebuildPromiseRef.current === rebuildPromise) {
          rebuildPromiseRef.current = null;
        }
      };
      void rebuildPromise.then(clearRebuild, clearRebuild);
      return rebuildPromise;
    },
    [buildConnection, markInterrupted],
  );

  const retryConnection = useCallback(() => {
    if (retryPromiseRef.current) {
      return retryPromiseRef.current;
    }

    if (isBrowserOffline()) {
      markInterrupted('offline');
      return Promise.reject(new Error('Cannot reconnect while the browser is offline'));
    }

    const activeConnection = activeConnectionRef.current;
    if (activeConnection?.state === HubConnectionState.Connected) {
      markConnected();
      return Promise.resolve();
    }

    if (startInProgressRef.current && readyPromiseRef.current) {
      return readyPromiseRef.current.then(() => undefined);
    }

    const retryPromise = rebuildConnection(true).then(() => undefined);
    retryPromiseRef.current = retryPromise;

    const clearRetry = () => {
      if (retryPromiseRef.current === retryPromise) {
        retryPromiseRef.current = null;
      }
    };
    void retryPromise.then(clearRetry, clearRetry);
    return retryPromise;
  }, [markConnected, markInterrupted, rebuildConnection]);

  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  useEffect(() => {
    if (accessToken === prevTokenRef.current) {
      return;
    }

    prevTokenRef.current = accessToken;
    void rebuildConnection(interruptedAt !== undefined).catch(() => {});
  }, [accessToken, interruptedAt, rebuildConnection]);

  useEffect(() => {
    // Establishing the external HubConnection is this effect's synchronization responsibility.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void buildConnection().catch(() => {});

    return () => {
      rebuildGenerationRef.current += 1;
      if (activeCancelRef.current) {
        activeCancelRef.current.current = true;
      }

      const activeConnection = activeConnectionRef.current;
      activeConnectionRef.current = null;
      readyPromiseRef.current = null;
      rebuildPromiseRef.current = null;
      retryPromiseRef.current = null;
      startInProgressRef.current = false;
      activeConnection?.stop().catch(console.error);
    };
    // The connection is rebuilt explicitly when the access token changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const handleOffline = () => {
      markInterrupted('offline');
    };

    const handleOnline = () => {
      const activeConnection = activeConnectionRef.current;
      if (activeConnection?.state === HubConnectionState.Connected) {
        markConnected();
        return;
      }

      if (
        startInProgressRef.current ||
        activeConnection?.state === HubConnectionState.Connecting ||
        activeConnection?.state === HubConnectionState.Reconnecting
      ) {
        markInterrupted('reconnecting');
        return;
      }

      void retryConnection().catch(() => {});
    };

    window.addEventListener('offline', handleOffline);
    window.addEventListener('online', handleOnline);
    return () => {
      window.removeEventListener('offline', handleOffline);
      window.removeEventListener('online', handleOnline);
    };
  }, [markConnected, markInterrupted, retryConnection]);

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
    <SignalRContext.Provider
      value={{
        connection,
        connectionState,
        liveConnectionState,
        interruptedAt,
        lastConnectedAt,
        retryConnection,
        joinGroup,
        leaveGroup,
      }}>
      {children}
    </SignalRContext.Provider>
  );
};
