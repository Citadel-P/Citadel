import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/auth-context';
import { createSignalRConnection, SignalRConnectionFactory } from '../createSignalRConnection';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { LiveConnectionState, RealtimeContext } from './realtime-context';

type StartConnection = typeof startConnectionWithRetry;

type RealtimeProviderProps = {
  children?: React.ReactNode;
  connectionFactory?: SignalRConnectionFactory;
  startConnection?: StartConnection;
  realtimeTransport?: string;
  webSocketFactory?: (url: string) => WebSocket;
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
const createWebSocket = (url: string) => new WebSocket(url);

export const RealtimeProvider: React.FC<RealtimeProviderProps> = ({
  children,
  connectionFactory = createSignalRConnection,
  startConnection = startConnectionWithRetry,
  realtimeTransport,
  webSocketFactory = createWebSocket,
}) => {
  const [connectionState, setConnectionState] = useState<HubConnectionState>(HubConnectionState.Disconnected);
  const [liveConnectionState, setLiveConnectionState] = useState<LiveConnectionState>(
    realtimeTransport === 'SignalR' || realtimeTransport === 'WebSocketV1' ? 'connecting' : 'disconnected',
  );
  const [interruptedAt, setInterruptedAt] = useState<number>();
  const [lastConnectedAt, setLastConnectedAt] = useState<number>();
  const [connection, setConnection] = useState<HubConnection | null>(null);
  const { accessToken } = useAuthContext();
  const queryClient = useQueryClient();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const signalREnabled = realtimeTransport === 'SignalR';
  const webSocketEnabled = realtimeTransport === 'WebSocketV1';

  const tokenRef = useRef<string | undefined>(accessToken);
  const prevTokenRef = useRef<string | undefined>(accessToken);
  const activeCancelRef = useRef<CancellationRef | null>(null);
  const activeConnectionRef = useRef<HubConnection | null>(null);
  const readyPromiseRef = useRef<Promise<HubConnection> | null>(null);
  const rebuildPromiseRef = useRef<Promise<HubConnection> | null>(null);
  const retryPromiseRef = useRef<Promise<void> | null>(null);
  const startInProgressRef = useRef(false);
  const rebuildGenerationRef = useRef(0);
  const webSocketRef = useRef<WebSocket>();
  const [webSocketGeneration, setWebSocketGeneration] = useState(0);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  useEffect(() => {
    if (realtimeTransport !== 'WebSocketV1' || !accessToken) return;

    let disposed = false;
    let socket: WebSocket | undefined;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;
    let retryIndex = 0;
    const retryDelays = [0, 2_000, 5_000, 10_000, 30_000];
    const invalidateLicense = () => {
      void queryClient.invalidateQueries({ queryKey: ['getLicenseEntitlements'] });
      void queryClient.invalidateQueries({ queryKey: ['getLicense'] });
    };
    const invalidateResourceQueries = (resourceType?: string, eventKind?: string) => {
      if (resourceType === 'License' || eventKind === 'licenseStateChanged') {
        invalidateLicense();
        return;
      }
      const prefixes =
        eventKind === 'resourceTagsChanged'
          ? [
              'listTags',
              'listPlatforms',
              'listRegistries',
              'listGitRepositories',
              'getPlatfom',
              'getPlatformTags',
              'getRegistry',
              'getRegistryTags',
              'getGitRepository',
              'getGitRepositoryTags',
            ]
          : resourceType === 'Tag'
            ? ['listTags']
            : resourceType === 'Registry'
              ? ['listRegistries', 'getRegistry', 'getRegistryConfig', 'getRegistryTags', 'listActivities']
              : resourceType === 'GitRepository'
                ? [
                    'listGitRepositories',
                    'getGitRepository',
                    'getGitRepositoryConfig',
                    'getGitRepositoryTags',
                    'listActivities',
                  ]
                : resourceType === 'Binding'
                  ? ['getGlobalResourceBindings', 'getResourceBindings', 'listSecretDefinitions', 'listSecretProviders']
                  : [];
      if (prefixes.length === 0) return;
      void queryClient.invalidateQueries({
        predicate: (query) => prefixes.includes(String(query.queryKey[0])),
      });
    };
    const connect = () => {
      if (disposed) return;
      const url = new URL('/api/v1/realtime', baseUrl || window.location.origin);
      url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
      socket = webSocketFactory(url.toString());
      webSocketRef.current = socket;
      socket.addEventListener('open', () => {
        socket?.send(
          JSON.stringify({ protocolVersion: 1, kind: 'subscribe', accessToken }),
        );
      });
      socket.addEventListener('message', (event) => {
        try {
          const envelope = JSON.parse(String(event.data)) as {
            protocolVersion?: number;
            eventKind?: string;
            kind?: string;
            resourceType?: string;
          };
          if (envelope.protocolVersion === 1) {
            if (envelope.kind === 'subscribed') {
              retryIndex = 0;
              setConnectionState(HubConnectionState.Connected);
              setLiveConnectionState('connected');
              setInterruptedAt(undefined);
              setLastConnectedAt(Date.now());
              invalidateLicense();
            } else {
              invalidateResourceQueries(envelope.resourceType, envelope.eventKind);
            }
          }
        } catch {
          // A malformed notification is ignored; authoritative API reads remain unchanged.
        }
      });
      socket.addEventListener('close', () => {
        if (disposed) return;
        setConnectionState(HubConnectionState.Disconnected);
        setLiveConnectionState(isBrowserOffline() ? 'offline' : 'reconnecting');
        setInterruptedAt((current) => current ?? Date.now());
        const delay = retryDelays[Math.min(retryIndex, retryDelays.length - 1)];
        retryIndex += 1;
        retryTimer = setTimeout(connect, delay);
      });
    };

    connect();
    return () => {
      disposed = true;
      if (retryTimer) clearTimeout(retryTimer);
      if (webSocketRef.current === socket) webSocketRef.current = undefined;
      socket?.close();
    };
  }, [accessToken, baseUrl, queryClient, realtimeTransport, webSocketFactory, webSocketGeneration]);

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
      if (!signalREnabled) {
        return Promise.reject(new Error('SignalR transport is not enabled'));
      }

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
    [
      baseUrl,
      connectionFactory,
      markConnected,
      markInterrupted,
      queryClient,
      reconcileLiveQueries,
      signalREnabled,
      startConnection,
    ],
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
    if (webSocketEnabled) {
      if (isBrowserOffline()) {
        markInterrupted('offline');
        return Promise.reject(new Error('Cannot reconnect while the browser is offline'));
      }

      setConnectionState(HubConnectionState.Connecting);
      setLiveConnectionState('connecting');
      setWebSocketGeneration((current) => current + 1);
      return Promise.resolve();
    }

    if (!signalREnabled) {
      return Promise.reject(new Error('Realtime transport is not enabled'));
    }

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
  }, [markConnected, markInterrupted, rebuildConnection, signalREnabled, webSocketEnabled]);

  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  useEffect(() => {
    if (accessToken === prevTokenRef.current) {
      return;
    }

    prevTokenRef.current = accessToken;
    if (!signalREnabled) {
      return;
    }

    // Replacing the authenticated external connection is this effect's synchronization responsibility.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void rebuildConnection(interruptedAt !== undefined).catch(() => {});
  }, [accessToken, interruptedAt, rebuildConnection, signalREnabled]);

  useEffect(() => {
    if (!signalREnabled) {
      return;
    }

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
    // Connection inputs other than the negotiated transport are handled by the explicit
    // token-rebuild path or are fixed for the lifetime of the provider.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signalREnabled]);

  useEffect(() => {
    if (!signalREnabled && !webSocketEnabled) {
      return;
    }

    const handleOffline = () => {
      markInterrupted('offline');
    };

    const handleOnline = () => {
      if (webSocketEnabled) {
        setConnectionState(HubConnectionState.Connecting);
        setLiveConnectionState('connecting');
        setWebSocketGeneration((current) => current + 1);
        return;
      }

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
  }, [markConnected, markInterrupted, retryConnection, signalREnabled, webSocketEnabled]);

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
    <RealtimeContext.Provider
      value={{
        signalR: signalREnabled ? { connection, connectionState, joinGroup, leaveGroup } : undefined,
        liveConnectionState,
        interruptedAt,
        lastConnectedAt,
        retryConnection,
      }}>
      {children}
    </RealtimeContext.Provider>
  );
};
