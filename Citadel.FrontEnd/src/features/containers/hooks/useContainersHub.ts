import { useEffect, useState } from 'react';
import { HubConnection, HubConnectionBuilder, HubConnectionState, IHttpConnectionOptions } from '@microsoft/signalr';
import { useAuthContext } from '@/features/login/AuthProvider';
import { SignalrRetryPolicy } from '@/lib/signalr.retrypolicy';
import { ContainerInfoView, ContainersInfoView, ContainerStatView } from '@/api/_generated';

const useContainersHub = (platformId: string) => {
  const [containersInfo, setContainersInfo] = useState<ContainersInfoView | undefined>();
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
        setContainersInfo(msg);
      });
      hubConnection.on('ContainerEventReceived', (containerInfo: ContainerInfoView, eventType: string) => {
        // todo
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

  return { containersInfo };
};

export interface IContainerEvent {
  containerInfo: ContainerInfoView;
  eventType: string;
}

export default useContainersHub;
