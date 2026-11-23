import {
  GROUP_CAPACITY_ERROR,
  RealtimeConnection,
  RealtimeConnectionState,
  RealtimeConnectionFactory,
} from '../realtime-connection';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useAuthContext } from '@/features/auth/auth-context';
import { createRealtimeConnection, isRealtimeTransportEnabled } from '../createRealtimeConnection';
import { startConnectionWithRetry } from '../startConnectionWithRetry';
import { LiveConnectionState, RealtimeContext } from './realtime-context';

type StartConnection = typeof startConnectionWithRetry;

type RealtimeProviderProps = {
  children?: React.ReactNode;
  connectionFactory?: RealtimeConnectionFactory;
  startConnection?: StartConnection;
  realtimeTransport?: string;
  webSocketFactory?: (url: string) => WebSocket;
};

type GroupState = {
  state: 'pending' | 'joining' | 'joined' | 'capacity';
  references: number;
  joinedConnection?: RealtimeConnection;
  joinPromise?: Promise<void>;
};

type CancellationRef = { current: boolean };

// Details with full realtime snapshots refresh when their groups rejoin.
// Refetching them here can request a resource just deleted by the user.
const liveBackedQueryKeys = new Set([
  'getSetupStatus',
  'getServiceAccountLimits',
  'getApplicationInfo',
  'getInstanceUpdates',
  'getAgentUpdates',
  'getAgentUpdateOperation',
  'getInstanceUpdateOperation',
  'getInstallationSettings',
  'getInstanceRestart',
  'getDatabaseConnections',
  'getActivity',
  'getAlertEvent',
  'getProfileMfaStatus',
  'getBackupRepository',
  'getBuildRun',
  'getBuildRunLogs',
  'getBuildRunRecovery',
  'getContainerData',
  'getContainerNetworking',
  'listNetworkContainers',
  'inspectNetwork',
  'getContainersData',
  'getContainerStats',
  'getDeploymentStats',
  'getPlatformStats',
  'getPlatfom',
  'getStackStats',
  'getStackDrift',
  'listActivities',
  'listAlertEvents',
  'listScripts',
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
  'listTags',
  'listManagedSwarmServices',
  'listRegistries',
  'listStacks',
]);

const isBrowserOffline = () => typeof navigator !== 'undefined' && navigator.onLine === false;
const createWebSocket = (url: string) => new WebSocket(url);

export const RealtimeProvider: React.FC<RealtimeProviderProps> = ({
  children,
  connectionFactory,
  startConnection = startConnectionWithRetry,
  realtimeTransport,
  webSocketFactory = createWebSocket,
}) => {
  const [connectionState, setConnectionState] = useState<RealtimeConnectionState>(RealtimeConnectionState.Disconnected);
  const [liveConnectionState, setLiveConnectionState] = useState<LiveConnectionState>('connecting');
  const [interruptedAt, setInterruptedAt] = useState<number>();
  const [lastConnectedAt, setLastConnectedAt] = useState<number>();
  const [connection, setConnection] = useState<RealtimeConnection | null>(null);
  const { accessToken } = useAuthContext();
  const queryClient = useQueryClient();
  const baseUrl = import.meta.env.VITE_API_BASE_URL;
  const realtimeEnabled = isRealtimeTransportEnabled(realtimeTransport);

  const tokenRef = useRef<string | undefined>(accessToken);
  const prevTokenRef = useRef<string | undefined>(accessToken);
  const activeCancelRef = useRef<CancellationRef | null>(null);
  const activeConnectionRef = useRef<RealtimeConnection | null>(null);
  const readyPromiseRef = useRef<Promise<RealtimeConnection> | null>(null);
  const rebuildPromiseRef = useRef<Promise<RealtimeConnection> | null>(null);
  const retryPromiseRef = useRef<Promise<void> | null>(null);
  const startInProgressRef = useRef(false);
  const rebuildGenerationRef = useRef(0);
  const groupStates = useRef<Map<string, GroupState>>(new Map());

  const [capacityGroups, setCapacityGroups] = useState<ReadonlySet<string>>(() => new Set());
  const markCapacity = useCallback((name: string, limited: boolean) => {
    setCapacityGroups((current) => {
      if (current.has(name) === limited) return current;
      const next = new Set(current);
      if (limited) next.add(name);
      else next.delete(name);
      return next;
    });
  }, []);

  const requestGroupJoin = useCallback(
    (conn: RealtimeConnection, name: string, group: GroupState) => {
      if (group.joinPromise) return group.joinPromise;
      const retryingCapacity = group.state === 'capacity';
      group.state = 'joining';
      const current = () => groupStates.current.get(name) === group && activeConnectionRef.current === conn;
      const promise = conn
        .invoke('JoinGroup', name)
        .then(() => {
          if (current() && group.state === 'joining') {
            group.state = 'joined';
            group.joinedConnection = conn;
            markCapacity(name, false);
          }
        })
        .catch((error: unknown) => {
          const limited = error instanceof Error && error.message === GROUP_CAPACITY_ERROR;
          if (current()) {
            group.state = limited || retryingCapacity ? 'capacity' : 'pending';
            markCapacity(name, group.state === 'capacity');
          }
          // The subscription still owns its reference while the server cannot serve the snapshot.
          if (!limited) throw error;
        })
        .finally(() => {
          if (group.joinPromise === promise) group.joinPromise = undefined;
        });
      group.joinPromise = promise;
      return promise;
    },
    [markCapacity],
  );

  const markConnected = useCallback(() => {
    setConnectionState(RealtimeConnectionState.Connected);
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
      if (!realtimeEnabled) {
        return Promise.reject(new Error('Realtime transport is not enabled'));
      }

      if (activeCancelRef.current) {
        activeCancelRef.current.current = true;
      }

      const cancelRef: CancellationRef = { current: false };
      activeCancelRef.current = cancelRef;

      const options = { baseUrl, accessTokenFactory: () => tokenRef.current ?? '' };
      const conn = connectionFactory
        ? connectionFactory(options)
        : createRealtimeConnection(realtimeTransport, options, webSocketFactory);

      activeConnectionRef.current = conn;
      startInProgressRef.current = true;
      setConnectionState(RealtimeConnectionState.Connecting);
      if (isBrowserOffline()) {
        markInterrupted('offline');
      } else {
        setLiveConnectionState('connecting');
      }

      conn.on('RealtimeGroupCapacity', (name: string) => {
        const group = groupStates.current.get(name);
        if (group && activeConnectionRef.current === conn) {
          group.state = 'capacity';
          markCapacity(name, true);
        }
      });

      conn.on('ApplicationInfoChanged', () => {
        void queryClient.invalidateQueries({ queryKey: ['getApplicationInfo'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstanceUpdates'] });
        void queryClient.invalidateQueries({ queryKey: ['getAgentUpdates'] });
        void queryClient.invalidateQueries({ queryKey: ['getAgentUpdateOperation'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstanceUpdateOperation'] });
      });

      conn.on('InstallationSettingsChanged', () => {
        void queryClient.invalidateQueries({ queryKey: ['listActivities'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstallationSettings'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstanceRestart'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstanceUpdates'] });
        void queryClient.invalidateQueries({ queryKey: ['getAgentUpdates'] });
        void queryClient.invalidateQueries({ queryKey: ['getAgentUpdateOperation'] });
        void queryClient.invalidateQueries({ queryKey: ['getInstanceUpdateOperation'] });
        void queryClient.invalidateQueries({ queryKey: ['getApplicationInfo'] });
        void queryClient.invalidateQueries({ queryKey: ['getProfileMfaStatus'] });
      });

      conn.on('TagCatalogChanged', () => {
        void queryClient.invalidateQueries({ queryKey: ['listTags'] });
      });

      conn.on('LicenseStateChanged', () => {
        void queryClient.invalidateQueries({ queryKey: ['getLicenseEntitlements'] });
        void queryClient.invalidateQueries({ queryKey: ['getLicense'] });
      });

      conn.onreconnecting(() => {
        if (activeConnectionRef.current !== conn) {
          return;
        }

        for (const group of groupStates.current.values()) group.joinedConnection = undefined;
        setConnectionState(RealtimeConnectionState.Reconnecting);
        markInterrupted(isBrowserOffline() ? 'offline' : 'reconnecting');
      });

      const rejoinGroup = async (groupName: string) => {
        const group = groupStates.current.get(groupName);
        if (!group || activeConnectionRef.current !== conn || conn.state !== RealtimeConnectionState.Connected) {
          return;
        }

        try {
          await requestGroupJoin(conn, groupName, group);
        } catch (err) {
          console.error(`Failed to rejoin group ${groupName}:`, err);
          setTimeout(() => {
            if (groupStates.current.get(groupName) === group && group.state === 'pending') {
              void rejoinGroup(groupName);
            }
          }, 2_000);
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

        setConnectionState(RealtimeConnectionState.Disconnected);
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
            throw new Error('Realtime connection was replaced before it became ready');
          }

          if (conn.state !== RealtimeConnectionState.Connected) {
            throw new Error('Realtime connection did not reach the connected state');
          }

          setConnection(conn);
          markConnected();

          if (reconcileAfterConnect) {
            await reconcileLiveQueries();
          } else {
            // Cover changes between the initial HTTP read and the live subscription.
            await Promise.all(
              [
                'getSetupStatus',
                'getServiceAccountLimits',
                'getApplicationInfo',
                'getInstanceUpdates',
                'getAgentUpdates',
                'getAgentUpdateOperation',
                'getInstanceUpdateOperation',
                'getInstallationSettings',
                'getInstanceRestart',
                'getDatabaseConnections',
                'listActivities',
                'getProfileMfaStatus',
              ].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
            );
          }

          return conn;
        } catch (err) {
          if (!cancelRef.current && activeConnectionRef.current === conn) {
            console.error('[Realtime] final connection failure', err);
          }

          await conn.stop().catch(() => {});
          if (activeConnectionRef.current === conn) {
            setConnectionState(RealtimeConnectionState.Disconnected);
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
      realtimeTransport,
      webSocketFactory,
      markConnected,
      markInterrupted,
      markCapacity,
      requestGroupJoin,
      queryClient,
      reconcileLiveQueries,
      realtimeEnabled,
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
      setConnectionState(RealtimeConnectionState.Connecting);
      if (isBrowserOffline()) {
        markInterrupted('offline');
      } else {
        setLiveConnectionState('connecting');
      }

      const rebuildPromise = (async () => {
        if (currentConnection) {
          await currentConnection.stop().catch((err) => {
            console.warn('[Realtime] failed to stop replaced connection', err);
          });
        }

        if (rebuildGenerationRef.current !== rebuildGeneration) {
          throw new Error('Realtime connection rebuild was canceled');
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
    if (!realtimeEnabled) {
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
    if (activeConnection?.state === RealtimeConnectionState.Connected) {
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
  }, [markConnected, markInterrupted, rebuildConnection, realtimeEnabled]);

  useEffect(() => {
    tokenRef.current = accessToken;
  }, [accessToken]);

  useEffect(() => {
    if (accessToken === prevTokenRef.current) {
      return;
    }

    prevTokenRef.current = accessToken;
    if (!realtimeEnabled) {
      return;
    }

    // Replacing the authenticated external connection is this effect's synchronization responsibility.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void rebuildConnection(interruptedAt !== undefined).catch(() => {});
  }, [accessToken, interruptedAt, rebuildConnection, realtimeEnabled]);

  useEffect(() => {
    if (!realtimeEnabled) {
      return;
    }

    // Establishing the external RealtimeConnection is this effect's synchronization responsibility.
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
  }, [realtimeEnabled]);

  useEffect(() => {
    if (!realtimeEnabled) {
      return;
    }

    const handleOffline = () => {
      markInterrupted('offline');
    };

    const handleOnline = () => {
      const activeConnection = activeConnectionRef.current;
      if (activeConnection?.state === RealtimeConnectionState.Connected) {
        markConnected();
        return;
      }

      if (
        startInProgressRef.current ||
        activeConnection?.state === RealtimeConnectionState.Connecting ||
        activeConnection?.state === RealtimeConnectionState.Reconnecting
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
  }, [markConnected, markInterrupted, retryConnection, realtimeEnabled]);

  const ensureConnectionReady = useCallback(async () => {
    const activeConnection = activeConnectionRef.current;
    if (activeConnection?.state === RealtimeConnectionState.Connected) {
      return activeConnection;
    }

    const readyConnection = await readyPromiseRef.current;
    if (readyConnection?.state === RealtimeConnectionState.Connected) {
      return readyConnection;
    }

    throw new Error('Realtime connection not available');
  }, []);

  const joinGroup = useCallback(
    async (groupName: string, setup?: (hub: RealtimeConnection) => void) => {
      if (!groupName) {
        return;
      }

      const conn = await ensureConnectionReady();

      try {
        setup?.(conn);
      } catch (err) {
        console.error('setup callback threw:', err);
      }

      const group = groupStates.current.get(groupName) ?? { state: 'pending', references: 0 };
      group.references++;
      groupStates.current.set(groupName, group);
      if (group.state === 'joined' && group.joinedConnection === conn) return;
      try {
        await requestGroupJoin(conn, groupName, group);
      } catch (err) {
        console.error('JoinGroup failed:', err);
        if (--group.references === 0 && groupStates.current.get(groupName) === group) {
          groupStates.current.delete(groupName);
          markCapacity(groupName, false);
        }
        throw err;
      }
    },
    [ensureConnectionReady, markCapacity, requestGroupJoin],
  );

  const isGroupLive = useCallback((name: string) => {
    const connection = activeConnectionRef.current;
    const group = groupStates.current.get(name);
    return (
      connection?.state === RealtimeConnectionState.Connected &&
      group?.state === 'joined' &&
      group.joinedConnection === connection
    );
  }, []);

  const retryGroup = useCallback(
    async (name: string) => {
      const conn = activeConnectionRef.current;
      const group = groupStates.current.get(name);
      if (conn?.state === RealtimeConnectionState.Connected && group?.state === 'capacity') {
        await requestGroupJoin(conn, name, group);
      }
    },
    [requestGroupJoin],
  );

  const leaveGroup = useCallback(
    async (groupName: string, remove?: (hub: RealtimeConnection) => void) => {
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
        group.references--;
        return;
      }

      groupStates.current.delete(groupName);
      markCapacity(groupName, false);

      if (connection?.state === RealtimeConnectionState.Connected) {
        try {
          await connection.invoke('LeaveGroup', groupName);
        } catch (err) {
          console.warn('LeaveGroup failed:', err);
        }
      }
    },
    [connection, markCapacity],
  );

  return (
    <RealtimeContext.Provider
      value={{
        groups: realtimeEnabled
          ? { connection, connectionState, joinGroup, leaveGroup, retryGroup, capacityGroups, isGroupLive }
          : undefined,
        // Application-info discovery is startup, not a failed connection.
        liveConnectionState:
          realtimeTransport === undefined ? 'connecting' : realtimeEnabled ? liveConnectionState : 'disconnected',
        interruptedAt,
        lastConnectedAt,
        retryConnection,
      }}>
      {children}
    </RealtimeContext.Provider>
  );
};
