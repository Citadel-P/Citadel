import { useEffect, useState } from 'react';
import { PlatformView } from '@/api/_generated';
import { HubConnection, HubConnectionBuilder, IHttpConnectionOptions } from '@microsoft/signalr';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { useContextSelector } from 'use-context-selector';
import { AuthContext } from '@/features/auth/AuthProvider';

export enum ConnectionState {
  unknown,
  connecting,
  connected,
}

const usePlatformHub = () => {
  const [connectionState, setConnectionState] = useState(ConnectionState.unknown);
  const [platformsMessage, setPlatformsMessage] = useState<PlatformView[] | undefined>();
  const accessToken = useContextSelector(AuthContext, (v) => v?.accessToken);

  useEffect(() => {
    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const baseUrl = import.meta.env.VITE_API_BASE_URL;

      const httpOptions: IHttpConnectionOptions = {
        accessTokenFactory: () => accessToken!,
      };
      hubConnection = new HubConnectionBuilder()
        .withUrl(`${baseUrl}/hubs/platform`, httpOptions)
        .withAutomaticReconnect(new SignalrRetryPolicy())
        .build();

      startConnection();
    };

    const startConnection = async () => {
      hubConnection.onreconnecting(() => setConnectionState(ConnectionState.connecting));
      hubConnection.onreconnected(() => onConnected());

      await hubConnection
        .start()
        .then((_) => onConnected())
        .catch((_) => {
          if (!isCanceled) setTimeout(() => startConnection(), 10000);
          else hubConnection.stop();
        });
    };

    const onConnected = () => {
      setConnectionState(ConnectionState.connected);
      hubConnection.on('PlatformsUpdated', (platforms: PlatformView[]) => {
        setPlatformsMessage(platforms);
      });
    };

    initHub();

    return () => {
      isCanceled = true;
      hubConnection.stop();
    };
  }, [accessToken]);

  return { connectionState, platformsMessage };
};

export default usePlatformHub;
