import { useEffect, useState } from 'react';
import { PlatformView } from '@/api/_generated';
import { HubConnection, HubConnectionBuilder, IHttpConnectionOptions } from '@microsoft/signalr';
import { useAuthContext } from '../../login/AuthProvider';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';

export enum ConnectionState {
  unknown,
  connecting,
  connected,
}

const usePlatformHub = () => {
  const [connectionState, setConnectionState] = useState(ConnectionState.unknown);
  const [platformMessage, setPlatformMessage] = useState<PlatformView | undefined>();
  const { jwtToken } = useAuthContext();

  useEffect(() => {
    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const baseUrl = import.meta.env.VITE_API_BASE_URL;

      const httpOptions: IHttpConnectionOptions = {
        accessTokenFactory: () => jwtToken,
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
        .catch((_) => setTimeout(() => startConnection(), 10000));
    };

    const onConnected = () => {
      if (isCanceled) return hubConnection.stop();

      setConnectionState(ConnectionState.connected);
      hubConnection.on('PlatformUpdated', (platform: PlatformView) => {
        setPlatformMessage(platform);
      });
    };

    initHub();

    return () => {
      isCanceled = true;
      hubConnection.stop();
    };
  }, [jwtToken]);

  return { connectionState, platformMessage };
};

export default usePlatformHub;
