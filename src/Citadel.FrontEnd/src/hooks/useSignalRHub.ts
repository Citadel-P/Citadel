import { useEffect, useRef } from 'react';
import { HubConnection, HubConnectionBuilder, HttpTransportType } from '@microsoft/signalr';
import { startConnectionWithRetry } from '@/lib/startConnectionWithRetry';

type SignalRHookOptions = {
  url: string;
  accessToken: string | undefined;
  groupName: string | undefined;
  setupEventListeners: (hub: HubConnection) => void;
  removeEventListeners?: (hub: HubConnection) => void;
  onConnected?: (hub: HubConnection) => void;
  retryOptions?: {
    maxRetries?: number;
    initialRetryDelayMs?: number;
    maxRetryDelayMs?: number;
    jitterFactor?: number;
    onRetryAttempt?: (attempt: number, delay: number, error: Error) => void;
  };
};

export const useSignalRHub = ({
  url,
  accessToken,
  groupName,
  setupEventListeners,
  removeEventListeners,
  onConnected,
  retryOptions,
}: SignalRHookOptions) => {
  const isCanceledRef = useRef(false);
  const connectionRef = useRef<HubConnection | null>(null);

  useEffect(() => {
    if (!accessToken || !groupName) return;
    isCanceledRef.current = false;

    const connect = async () => {
      const hub = new HubConnectionBuilder()
        .withUrl(url, {
          accessTokenFactory: () => accessToken,
          transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
        })
        .withAutomaticReconnect()
        .build();

      setupEventListeners(hub);

      try {
        await startConnectionWithRetry(hub, isCanceledRef, retryOptions);
        if (isCanceledRef.current) {
          await hub.stop();
          return;
        }

        await hub.send('JoinGroup', groupName);
        onConnected?.(hub);
        connectionRef.current = hub;
      } catch (err) {
        console.error('SignalR final connection failure:', err);
        await hub.stop();
      }
    };

    connect();

    return () => {
      isCanceledRef.current = true;
      const conn = connectionRef.current;
      connectionRef.current = null;

      if (conn) {
        (async () => {
          try {
            if (conn.state === 'Connected') {
              await conn.send('LeaveGroup', groupName);
            }
            removeEventListeners?.(conn);
            await conn.stop();
          } catch (err) {
            console.warn('SignalR cleanup failed:', err);
          }
        })();
      }
    };
  }, [url, accessToken, groupName, retryOptions, setupEventListeners, removeEventListeners, onConnected]);
};
