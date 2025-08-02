import { useInterval } from '@/hooks/useInterval';
import { useGETAccessToken } from './useGETAccessToken';
import { jwtDecode } from 'jwt-decode';
import { useEffect, useRef } from 'react';

export const useTokenRefresh = (accessToken: string | undefined, isAuthenticated: boolean) => {
  const { refetch: refetchAccessToken } = useGETAccessToken();
  const isRefreshing = useRef(false);

  const parseJwt = (token: string) => {
    try {
      if (!token) return null;
      return jwtDecode(token);
    } catch (error) {
      console.error('Failed to parse JWT:', error);
      return null;
    }
  };

  useInterval(
    () => {
      if (isAuthenticated && accessToken && !isRefreshing.current) {
        const result = parseJwt(accessToken);

        if (result?.exp) {
          const currentTime = Math.floor(Date.now() / 1000);
          const timeToExpire = result.exp - currentTime;
          if (timeToExpire < 30) {
            isRefreshing.current = true;
            refetchAccessToken().finally(() => {
              isRefreshing.current = false;
            });
          }
        }
      }
    },
    isAuthenticated ? 10000 : null,
  );

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        if (isAuthenticated && accessToken && !isRefreshing.current) {
          const result = parseJwt(accessToken);

          if (result?.exp) {
            const currentTime = Math.floor(Date.now() / 1000);
            const timeToExpire = result.exp - currentTime;
            if (timeToExpire < 10) {
              isRefreshing.current = true;
              refetchAccessToken().finally(() => {
                isRefreshing.current = false;
              });
            }
          }
        }
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [accessToken, isAuthenticated, refetchAccessToken]);
};
