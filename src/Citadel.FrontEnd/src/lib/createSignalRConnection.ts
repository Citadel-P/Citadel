import {
  HubConnection,
  HubConnectionBuilder,
  HttpTransportType,
  IHttpConnectionOptions,
} from '@microsoft/signalr';
import { MessagePackHubProtocol } from '@microsoft/signalr-protocol-msgpack';

export type SignalRConnectionFactoryOptions = {
  baseUrl: string;
  accessTokenFactory: () => string;
};

export type SignalRConnectionFactory = (options: SignalRConnectionFactoryOptions) => HubConnection;

export const createSignalRConnection: SignalRConnectionFactory = ({ baseUrl, accessTokenFactory }) =>
  new HubConnectionBuilder()
    .withUrl(`${baseUrl}/hubs/global`, {
      accessTokenFactory,
      transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
    } as IHttpConnectionOptions)
    .withAutomaticReconnect()
    .withHubProtocol(new MessagePackHubProtocol())
    .build();
