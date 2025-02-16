import { useEffect, useState } from 'react';
import { HubConnection, HubConnectionBuilder, HubConnectionState, IHttpConnectionOptions } from '@microsoft/signalr';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { ContainerInfoView, ContainersInfoView } from '@/api/_generated';
import { useApiClientContext } from '@/api/ApiClientProvider';

const useContainersHub = (platformId: string) => {
  const [containersInfo, setContainersInfo] = useState<ContainersInfoView | undefined>();
  const { accessToken } = useApiClientContext();
  const groupName = `ContainersInfo/${platformId}`;

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
      hubConnection.on('ContainersInfoUpdated', (msg: ContainersInfoView) => {
        setContainersInfo(msg);
      });
      hubConnection.on('ContainerEventReceived', (containerInfo: ContainerInfoView, eventType: string) => {
        // todo
      });
    };

    initHub();

    return () => {
      if (hubConnection.state === HubConnectionState.Connected) hubConnection.send('LeaveGroup', groupName);

      isCanceled = true;
      hubConnection.stop();
    };
  }, [accessToken, platformId, groupName]);

  return { containersInfo };
};

export interface IContainerEvent {
  containerInfo: ContainerInfoView;
  eventType: string;
}

export default useContainersHub;
