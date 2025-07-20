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

export interface ISignalrRetryPolicyOptions {
  maxAttempts?: number;
  initialDelayMs?: number;
  maxDelayMs?: number;
  jitterFactor?: number;
}

/**
 * Custom SignalR retry policy implementing exponential backoff with jitter.
 */
export class SignalrRetryPolicy implements IRetryPolicy {
  private readonly maxAttempts: number;
  private readonly initialDelayMs: number;
  private readonly maxDelayMs: number;
  private readonly jitterFactor: number;

  constructor(options?: ISignalrRetryPolicyOptions) {
    this.maxAttempts = options?.maxAttempts ?? 10;
    this.initialDelayMs = options?.initialDelayMs ?? 1000;
    this.maxDelayMs = options?.maxDelayMs ?? 30000;
    this.jitterFactor = options?.jitterFactor ?? 0.3;
  }

  nextRetryDelayInMilliseconds(retryContext: RetryContext): number | null {
    const { elapsedMilliseconds, previousRetryCount, retryReason } = retryContext;

    console.warn(
      `SignalR connection retry attempt ${previousRetryCount + 1}/${this.maxAttempts}. ` +
        `Reason: ${retryReason || 'Unknown error'}. ` +
        `Elapsed time: ${elapsedMilliseconds / 1000}s.`,
    );

    if (previousRetryCount >= this.maxAttempts) {
      console.error(`SignalR: Max retry attempts (${this.maxAttempts}) reached. Stopping retries.`);
      return null;
    }

    const baseDelay = this.initialDelayMs * Math.pow(2, previousRetryCount);
    let delay = Math.min(baseDelay, this.maxDelayMs);

    if (this.jitterFactor > 0) {
      const randomJitter = Math.random() * this.jitterFactor * delay;
      delay += (Math.random() > 0.5 ? 1 : -1) * randomJitter;
      delay = Math.max(this.initialDelayMs, delay);
    }

    console.info(`SignalR: Next retry in ${delay / 1000} seconds.`);
    return delay;
  }
}

/**
 * Configures and builds a SignalR HubConnection.
 *
 * @param config IHubConfig object containing URL and access token string.
 * @param retryPolicyOptions Optional configuration for the SignalR retry policy.
 * @returns A configured HubConnection instance.
 */
export const configureHub = (
  { url, accessToken }: IHubConfig,
  retryPolicyOptions?: ISignalrRetryPolicyOptions,
): HubConnection => {
  const httpOptions: IHttpConnectionOptions = {
    accessTokenFactory: () => accessToken,
    transport: HttpTransportType.WebSockets | HttpTransportType.LongPolling,
  };

  const retryPolicy = new SignalrRetryPolicy(retryPolicyOptions);

  return new HubConnectionBuilder().withUrl(url, httpOptions).withAutomaticReconnect(retryPolicy).build();
};

/**
 * Starts a SignalR HubConnection with exponential backoff and retry logic.
 *
 * @param hubConnection The SignalR HubConnection instance.
 * @param onConnected Callback function to execute once the connection is successfully established.
 * @param isCanceled A ref object to signal cancellation of the retry attempts.
 * @param options Configuration options for retry behavior.
 */
export const startConnectionWithRetry = async (
  hubConnection: HubConnection,
  onConnected: () => void,
  isCanceled: { current: boolean },
  options?: {
    maxRetries?: number;
    initialRetryDelayMs?: number; // Starting delay for exponential backoff
    maxRetryDelayMs?: number; // Maximum delay for exponential backoff
    jitterFactor?: number; // Factor for adding randomness to the delay (0 to 1)
    onRetryAttempt?: (attempt: number, delay: number, error: Error) => void; // Callback for each retry attempt
  },
  currentRetryAttempt = 0, // Internal counter for retry attempts
): Promise<void> => {
  const {
    maxRetries = 10, // Default to 10 retries
    initialRetryDelayMs = 1000, // 1 second
    maxRetryDelayMs = 30000, // 30 seconds
    jitterFactor = 0.3, // 30% jitter
    onRetryAttempt,
  } = options || {};

  try {
    await hubConnection.start();
    onConnected();
    console.log('SignalR connection established successfully.');
  } catch (error: any) {
    if (isCanceled.current) {
      console.warn('Connection attempt canceled. Stopping hub connection.');
      hubConnection.stop(); // Stop the connection if cancellation is requested
      return;
    }

    if (currentRetryAttempt >= maxRetries) {
      console.error(`Max retry attempts (${maxRetries}) reached. Could not establish SignalR connection.`, error);
      // Optionally, we might want to stop the connection here or throw an error
      // hubConnection.stop();
      return;
    }

    const delayBase = initialRetryDelayMs * Math.pow(2, currentRetryAttempt);
    let retryDelay = Math.min(delayBase, maxRetryDelayMs);

    // Add jitter
    if (jitterFactor > 0) {
      const randomDelay = Math.random() * jitterFactor * retryDelay;
      retryDelay += (Math.random() > 0.5 ? 1 : -1) * randomDelay; // Add or subtract random amount
      retryDelay = Math.max(initialRetryDelayMs, retryDelay); // Ensure minimum delay
    }

    console.warn(
      `SignalR connection failed. Retrying in ${
        retryDelay / 1000
      } seconds (Attempt ${currentRetryAttempt + 1}/${maxRetries}).`,
      error,
    );

    if (onRetryAttempt) {
      onRetryAttempt(currentRetryAttempt + 1, retryDelay, error);
    }

    setTimeout(
      () => startConnectionWithRetry(hubConnection, onConnected, isCanceled, options, currentRetryAttempt + 1),
      retryDelay,
    );
  }
};
