import { useCallback, useEffect, useRef, useState } from 'react';
import { jwtDecode } from 'jwt-decode';
import { useInterval } from '@/hooks/useInterval';

const EXPIRY_BUFFER_INTERVAL = 30; // seconds before expiry to refresh
const EXPIRY_BUFFER_ON_FOCUS = 10; // when tab becomes visible

const parseJwt = (token: string) => {
  try {
    return token ? (jwtDecode(token) as { exp?: number }) : null;
  } catch {
    return null;
  }
};

const getTokenSecondsLeft = (token: string | undefined) => {
  if (!token) return undefined;
  const decoded = parseJwt(token);
  if (!decoded?.exp) return undefined;

  return decoded.exp - Math.floor(Date.now() / 1000);
};

export const useTokenRefresh = (
  accessToken: string | undefined,
  isAuthenticated: boolean,
  refreshToken: () => Promise<string | undefined>,
  onTokenRefreshed: (token: string) => void,
) => {
  const refreshPromise = useRef<Promise<string | undefined> | null>(null);
  const didAttemptRefresh = useRef(false);
  const [error, setError] = useState<unknown>();

  /** Manual refresh trigger (returns token if successful) */
  const triggerManualRefresh = useCallback(async (): Promise<string | undefined> => {
    if (refreshPromise.current) {
      return refreshPromise.current;
    }

    didAttemptRefresh.current = true;

    refreshPromise.current = refreshToken()
      .then((token) => {
        setError(undefined);
        if (token) {
          onTokenRefreshed(token);
        }
        return token;
      })
      .catch((err) => {
        console.error('refresh error:', err);
        setError(err);
        return undefined;
      })
      .finally(() => {
        refreshPromise.current = null;
      });

    return refreshPromise.current;
  }, [onTokenRefreshed, refreshToken]);

  /** Periodically check expiry and refresh early */
  const maybeRefreshIfExpiringSoon = useCallback(
    async (bufferSeconds: number) => {
      if (!isAuthenticated || !accessToken) return;

      const secondsLeft = getTokenSecondsLeft(accessToken);
      if (secondsLeft === undefined) return;

      if (secondsLeft < bufferSeconds) {
        await triggerManualRefresh();
      }
    },
    [accessToken, isAuthenticated, triggerManualRefresh],
  );

  // Periodic refresh check
  useInterval(() => maybeRefreshIfExpiringSoon(EXPIRY_BUFFER_INTERVAL), isAuthenticated ? 10_000 : null);

  // Refresh on tab focus
  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void maybeRefreshIfExpiringSoon(EXPIRY_BUFFER_ON_FOCUS);
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => document.removeEventListener('visibilitychange', onVisible);
  }, [maybeRefreshIfExpiringSoon]);

  return {
    error,
    didAttemptRefresh,
    triggerManualRefresh,
    isAccessTokenExpired: (getTokenSecondsLeft(accessToken) ?? 0) <= 0,
  };
};
