import { toast } from 'sonner';
import { canShowCachedResource, notifyRequestError, requestErrorMessage } from './request-error';

describe('request failure feedback', () => {
  it('keeps field validation details and provides a network fallback', () => {
    expect(requestErrorMessage({ status: 400, error: { errors: { name: ['Name is required.'] } } })).toBe(
      'Name is required.',
    );
    expect(requestErrorMessage(new TypeError('Failed to fetch'))).toContain('Check your connection');
    expect(canShowCachedResource({ status: 403 })).toBe(false);
    expect(canShowCachedResource({ status: 503 })).toBe(true);
  });

  it('deduplicates the same rejection but reports a later failed attempt', () => {
    vi.useFakeTimers();
    try {
      const notice = vi.spyOn(toast, 'error');
      const error = new TypeError('Failed to fetch');
      notifyRequestError(error);
      notifyRequestError(error);
      expect(notice).toHaveBeenCalledTimes(1);
      vi.runAllTimers();
      notifyRequestError(error);
      expect(notice).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
