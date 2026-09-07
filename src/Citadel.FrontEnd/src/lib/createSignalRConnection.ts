import { HubConnectionBuilder, HttpTransportType, IHttpConnectionOptions } from '@microsoft/signalr';
import { MessagePackHubProtocol } from '@microsoft/signalr-protocol-msgpack';

import { RealtimeConnectionFactory } from './realtime-connection';

export const createSignalRConnection: RealtimeConnectionFactory = ({ baseUrl, accessTokenFactory }) =>
  new HubConnectionBuilder()
    .withUrl(`${baseUrl}/hubs/global`, {
      accessTokenFactory,
      transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
    } as IHttpConnectionOptions)
    .withAutomaticReconnect([0, 2_000, 5_000, 10_000, 30_000])
    .withHubProtocol(new MessagePackHubProtocol())
    .build();
