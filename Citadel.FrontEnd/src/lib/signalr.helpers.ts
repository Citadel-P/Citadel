import {
  HttpTransportType,
  HubConnection,
  HubConnectionBuilder,
  IHttpConnectionOptions,
  IRetryPolicy,
  RetryContext,
} from '@microsoft/signalr';

export interface IHubConfig {
  url: string;
  accessToken: string;
}
export const configureHub = ({ url, accessToken }: IHubConfig): HubConnection => {
  const httpOptions: IHttpConnectionOptions = {
    accessTokenFactory: () => accessToken,
    transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
  };

  return new HubConnectionBuilder().withUrl(url, httpOptions).withAutomaticReconnect(new SignalrRetryPolicy()).build();
};

export const startConnectionWithRetry = async (
  hubConnection: HubConnection,
  onConnected: () => void,
  isCanceled: boolean,
): Promise<void> => {
  try {
    await hubConnection.start();
    onConnected();
  } catch {
    if (!isCanceled) {
      setTimeout(() => startConnectionWithRetry(hubConnection, onConnected, isCanceled), 10000);
    } else {
      hubConnection.stop();
    }
  }
};

export class SignalrRetryPolicy implements IRetryPolicy {
  nextRetryDelayInMilliseconds(retryContext: RetryContext): number {
    return 15 * 1000; // 15s
  }
}
