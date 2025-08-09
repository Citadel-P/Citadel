import { useEffect, useRef } from 'react';
import { HubConnection, HubConnectionBuilder, HttpTransportType, IHttpConnectionOptions } from '@microsoft/signalr';
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
  skip?: boolean;
};

export const useSignalRHub = ({
  url,
  accessToken,
  groupName,
  setupEventListeners,
  removeEventListeners,
  onConnected,
  retryOptions,
  skip,
}: SignalRHookOptions) => {
  const isCanceledRef = useRef(false);
  const connectionRef = useRef<HubConnection | null>(null);

  useEffect(() => {
    if (skip) {
      return;
    }

    isCanceledRef.current = false;
    let active = true;

    const hub = new HubConnectionBuilder()
      .withUrl(url, {
        accessTokenFactory: () => accessToken,
        transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
      } as IHttpConnectionOptions)
      .withAutomaticReconnect()
      .build();

    setupEventListeners(hub);

    (async () => {
      try {
        await startConnectionWithRetry(hub, isCanceledRef, retryOptions);

        if (!active || isCanceledRef.current) {
          await hub.stop();
          return;
        }

        if (groupName) {
          await hub.send('JoinGroup', groupName);
          console.log('JoinGroup:', groupName);
        }
        onConnected?.(hub);
        connectionRef.current = hub;
      } catch (err) {
        console.error('SignalR final connection failure:', err);
        await hub.stop();
      }
    })();

    return () => {
      if (skip) {
        return;
      }
      isCanceledRef.current = true;
      active = false;

      const conn = connectionRef.current;
      connectionRef.current = null;

      if (conn) {
        (async () => {
          try {
            if (groupName) {
              await conn.send('LeaveGroup', groupName);
            }
          } catch (err) {
            console.warn('LeaveGroup failed:', err);
          }

          try {
            removeEventListeners?.(conn);
            await conn.stop();
            console.log('Connection stopped');
          } catch (err) {
            console.warn('Cleanup failed:', err);
          }
        })();
      }
    };
  }, [url, accessToken, groupName, retryOptions, setupEventListeners, removeEventListeners, onConnected, skip]);
};
