import { HubConnection } from "@microsoft/signalr";

export const startConnectionWithRetry = async (
  connection: HubConnection,
  isCanceled: { current: boolean },
  options?: {
    maxRetries?: number;
    initialRetryDelayMs?: number;
    maxRetryDelayMs?: number;
    jitterFactor?: number;
    onRetryAttempt?: (attempt: number, delay: number, error: Error) => void;
  },
  currentRetryAttempt = 0,
): Promise<void> => {
  const {
    maxRetries = 10,
    initialRetryDelayMs = 1000,
    maxRetryDelayMs = 30000,
    jitterFactor = 0.3,
    onRetryAttempt,
  } = options || {};

  try {
    await connection.start();
    console.log('SignalR connection established');
  } catch (error: any) {
    if (isCanceled.current) {
      if (connection.state === 'Disconnected') {
        console.warn('Retry canceled');
        await connection.stop();
      }
      return;
    }

    if (currentRetryAttempt >= maxRetries) {
      console.error('Max retries reached:', error);
      return;
    }

    const delayBase = initialRetryDelayMs * Math.pow(2, currentRetryAttempt);
    let retryDelay = Math.min(delayBase, maxRetryDelayMs);

    if (jitterFactor > 0) {
      const randomDelay = Math.random() * jitterFactor * retryDelay;
      retryDelay += (Math.random() > 0.5 ? 1 : -1) * randomDelay;
      retryDelay = Math.max(initialRetryDelayMs, retryDelay);
    }

    console.warn(
      `SignalR connection failed. Retrying in ${(retryDelay / 1000).toFixed(1)}s (Attempt ${currentRetryAttempt + 1})`,
      error,
    );

    onRetryAttempt?.(currentRetryAttempt + 1, retryDelay, error);

    setTimeout(
      () =>
        startConnectionWithRetry(connection, isCanceled, options, currentRetryAttempt + 1),
      retryDelay,
    );
  }
};