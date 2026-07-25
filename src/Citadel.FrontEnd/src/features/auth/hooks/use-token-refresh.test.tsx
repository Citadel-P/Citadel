import { act, renderHook, waitFor } from '@testing-library/react';
import { useTokenRefresh } from './use-token-refresh';

const createToken = (expiresAt: number) => {
  const encode = (value: object) =>
    btoa(JSON.stringify(value)).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');

  return `${encode({ alg: 'none', typ: 'JWT' })}.${encode({ exp: expiresAt })}.signature`;
};

describe('useTokenRefresh', () => {
  it('deduplicates concurrent refresh requests', async () => {
    let resolveRefresh: ((token: string) => void) | undefined;
    const refreshToken = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          resolveRefresh = resolve;
        }),
    );
    const onTokenRefreshed = vi.fn();
    const { result } = renderHook(() =>
      useTokenRefresh(undefined, false, refreshToken, onTokenRefreshed),
    );

    let firstRefresh: Promise<string | undefined>;
    let secondRefresh: Promise<string | undefined>;
    act(() => {
      firstRefresh = result.current.triggerManualRefresh();
      secondRefresh = result.current.triggerManualRefresh();
    });

    expect(refreshToken).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveRefresh?.('new-access-token');
      await Promise.all([firstRefresh!, secondRefresh!]);
    });

    expect(onTokenRefreshed).toHaveBeenCalledOnce();
    expect(onTokenRefreshed).toHaveBeenCalledWith('new-access-token');
  });

  it('refreshes an expiring token when the page becomes visible', async () => {
    const accessToken = createToken(Math.floor(Date.now() / 1000) + 5);
    const refreshToken = vi.fn().mockResolvedValue('refreshed-token');
    const onTokenRefreshed = vi.fn();

    renderHook(() => useTokenRefresh(accessToken, true, refreshToken, onTokenRefreshed));

    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    await waitFor(() => {
      expect(refreshToken).toHaveBeenCalledOnce();
    });
    expect(onTokenRefreshed).toHaveBeenCalledWith('refreshed-token');
  });

  it('keeps a valid token when the page becomes visible', async () => {
    const accessToken = createToken(Math.floor(Date.now() / 1000) + 300);
    const refreshToken = vi.fn().mockResolvedValue('unused-token');

    renderHook(() => useTokenRefresh(accessToken, true, refreshToken, vi.fn()));

    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    await Promise.resolve();
    expect(refreshToken).not.toHaveBeenCalled();
  });

  it('reports missing, malformed, and expired tokens as expired', () => {
    const refreshToken = vi.fn().mockResolvedValue(undefined);
    const { result, rerender } = renderHook(
      ({ token }) => useTokenRefresh(token, false, refreshToken, vi.fn()),
      { initialProps: { token: undefined as string | undefined } },
    );

    expect(result.current.isAccessTokenExpired).toBe(true);

    rerender({ token: 'not-a-jwt' });
    expect(result.current.isAccessTokenExpired).toBe(true);

    rerender({ token: createToken(Math.floor(Date.now() / 1000) - 1) });
    expect(result.current.isAccessTokenExpired).toBe(true);
  });
});
