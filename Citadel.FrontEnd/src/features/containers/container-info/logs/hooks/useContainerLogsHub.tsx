import { useEffect, useState } from 'react';
import { HubConnection, HubConnectionBuilder, HubConnectionState, IHttpConnectionOptions } from '@microsoft/signalr';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { ContainerLogsMessage } from '@/api/hub-models/containers-hub-models';
import { useApiClientContext } from '@/api/ApiClientProvider';

const useContainerLogsHub = (containerId: string, requestId: string) => {
  const [containerLogsMessage, setContainerLogsMessage] = useState<ContainerLogsMessage | undefined>();
  const { accessToken } = useApiClientContext();
  const groupName = `ContainerLogs/${containerId}/${requestId}`;

  useEffect(() => {
    let hubConnection: HubConnection;
    let isCanceled = false;

    const initHub = () => {
      const baseUrl = import.meta.env.VITE_API_BASE_URL;

      const httpOptions: IHttpConnectionOptions = {
        accessTokenFactory: () => accessToken!,
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
        .catch((_) => {
          if (!isCanceled) setTimeout(() => startConnection(), 10000);
          else hubConnection.stop();
        });
    };

    const onConnected = () => {
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
  }, [accessToken, containerId, groupName]);

  return { containerLogsMessage };
};

export default useContainerLogsHub;
