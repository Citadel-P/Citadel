import { RealtimeConnection } from '@/lib/realtime-connection';

/**
 * Starts a Realtime connection with a retry mechanism in case of failure.
 * Implements exponential backoff with jitter to prevent overwhelming the server.
 *
 * @param connection The Realtime hub connection to start.
 * @param isCanceled A ref object to signal cancellation of the retry process.
 * @param options Optional configuration for the retry mechanism.
 * @param {number} [options.maxRetries=10] The maximum number of retry attempts.
 * @param {number} [options.initialRetryDelayMs=1000] The initial delay in milliseconds for the first retry.
 * @param {number} [options.maxRetryDelayMs=30000] The maximum delay in milliseconds between retries.
 * @param {number} [options.jitterFactor=0.3] The factor to use for adding randomness to the retry delay (0 to 1).
 * @param {(attempt: number, delay: number, error: Error) => void} [options.onRetryAttempt] An optional callback that is executed before each retry attempt.
 * @param currentRetryAttempt The current retry attempt number (used for recursion).
 */
export const startConnectionWithRetry = async (
  connection: RealtimeConnection,
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

  if (isCanceled.current) {
    return;
  }

  try {
    // Attempt to start the connection.
    await connection.start();
  } catch (error: any) {
    // If the retry process has been canceled, stop the connection and exit.
    if (isCanceled.current) {
      if (connection.state === 'Disconnected') {
        console.warn('Retry process has been canceled.');
        await connection.stop();
      }
      return;
    }

    // If the maximum number of retries has been reached, log the error and exit.
    if (currentRetryAttempt >= maxRetries) {
      console.error('Maximum number of Realtime connection retries reached. Aborting.', error);
      throw error;
    }

    // Calculate the base delay for the next retry using exponential backoff.
    const delayBase = initialRetryDelayMs * Math.pow(2, currentRetryAttempt);
    // Cap the delay at the maximum retry delay.
    let retryDelay = Math.min(delayBase, maxRetryDelayMs);

    // Apply jitter to the retry delay to spread out connection attempts.
    if (jitterFactor > 0) {
      const randomDelay = Math.random() * jitterFactor * retryDelay;
      retryDelay += (Math.random() > 0.5 ? 1 : -1) * randomDelay;
      // Ensure the delay is not less than the initial delay.
      retryDelay = Math.max(initialRetryDelayMs, retryDelay);
    }

    console.warn(
      `Realtime connection failed. Retrying in ${(retryDelay / 1000).toFixed(1)}s (Attempt ${currentRetryAttempt + 1})`,
      error,
    );

    // Notify the caller about the retry attempt.
    onRetryAttempt?.(currentRetryAttempt + 1, retryDelay, error);

    // Schedule the next retry attempt.
    await new Promise<void>((resolve) => setTimeout(resolve, retryDelay));
    return startConnectionWithRetry(connection, isCanceled, options, currentRetryAttempt + 1);
  }
};
