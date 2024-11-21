import { useEffect, useState } from 'react';
import { HubConnection, HubConnectionBuilder, HubConnectionState, IHttpConnectionOptions } from '@microsoft/signalr';
import { useAuthContext } from '@/features/login/AuthProvider';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { ContainersInfoView } from '@/api/_generated';

const useContainersHub = (platformId: string) => {
  const [containersMessage, setContainersMessage] = useState<ContainersInfoView | undefined>();
  const { jwtToken } = useAuthContext();
  const groupName = `ContainersInfo/${platformId}`;

  useEffect(() => {
    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const baseUrl = import.meta.env.VITE_API_BASE_URL;

      const httpOptions: IHttpConnectionOptions = {
        accessTokenFactory: () => jwtToken,
      };
      hubConnection = new HubConnectionBuilder()
        .withUrl(`${baseUrl}/hubs/container`, httpOptions)
        .withAutomaticReconnect(new SignalrRetryPolicy())
        .build();

      startConnection();
    };

    const startConnection = async () => {
      hubConnection.onreconnected(() => onConnected());

      await hubConnection
        .start()
        .then((_) => onConnected())
        .catch((_) => setTimeout(() => startConnection(), 10000));
    };

    const onConnected = () => {
      if (isCanceled) return hubConnection.stop();
      hubConnection.send('JoinGroup', groupName);
      hubConnection.on('ContainersInfoUpdated', (msg: ContainersInfoView) => {
        setContainersMessage(msg);
      });
    };

    initHub();

    return () => {
      isCanceled = true;
      if (hubConnection.state === HubConnectionState.Connected) {
        hubConnection.send('LeaveGroup', groupName);
        hubConnection.stop();
      }
    };
  }, [jwtToken, platformId, groupName]);

  return { containersMessage };
};

export default useContainersHub;
