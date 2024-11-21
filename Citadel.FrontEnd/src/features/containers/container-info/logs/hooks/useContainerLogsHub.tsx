import { useEffect, useState } from 'react';
import { HubConnection, HubConnectionBuilder, HubConnectionState, IHttpConnectionOptions } from '@microsoft/signalr';
import { useAuthContext } from '@/features/login/AuthProvider';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { ContainerLogsMessage } from '@/api/hub-models/containers-hub-models';

const useContainerLogsHub = (containerId: string, requestId: string) => {
  const [containerLogsMessage, setContainerLogsMessage] = useState<ContainerLogsMessage | undefined>();
  const { jwtToken } = useAuthContext();
  const groupName = `ContainerLogs/${containerId}/${requestId}`;

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
      hubConnection.on('ContainerLogsReceived', (msg: ContainerLogsMessage) => {
        setContainerLogsMessage(msg);
      });
    };

    initHub();

    return () => {
      isCanceled = true;
      if (hubConnection.state === HubConnectionState.Connected) {
        hubConnection.send('LeaveGroup', groupName);
      }
      hubConnection.stop();
    };
  }, [jwtToken, containerId, groupName]);

  return { containerLogsMessage };
};

export default useContainerLogsHub;
