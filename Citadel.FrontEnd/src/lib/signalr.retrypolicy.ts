import { IRetryPolicy, RetryContext } from '@microsoft/signalr';

export class SignalrRetryPolicy implements IRetryPolicy {
  nextRetryDelayInMilliseconds(retryContext: RetryContext): number {
    return 15 * 1000; // 15s
  }
}
