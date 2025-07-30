import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import { useGETAccessToken } from './useGETAccessToken';
import { useApiClientContext } from '@/api/ApiClientContext';
import { useHTTPErrorHandler } from './useHTTPErrorHandler';
import { usePOSTLogout } from './usePOSTLogout';
import { useInterval } from '@/hooks/useInterval';
import { AuthContext } from './AuthContext';

const accessTokenKey = 'access_token';
const storedJwt = sessionStorage.getItem(accessTokenKey);

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { mutate: logout, isSuccess: logoutIsSuccess } = usePOSTLogout();
  const {
    data: accessTokenData,
    isSuccess,
    error: accessTokenError,
    refetch: refetchAccessToken,
  } = useGETAccessToken();
  const { apiClient } = useApiClientContext();
  const [accessToken, setAccessToken] = useState<string | undefined>(storedJwt ?? undefined);
  const isAuthenticated = useMemo(() => accessToken != null, [accessToken]);
  const isRefreshing = useRef(false);

  const handleSetAccessToken = useCallback(
    (token: string | undefined) => {
      if (token) {
        setAccessToken(token);
        apiClient?.setSecurityData(token);
        sessionStorage.setItem(accessTokenKey, token);
      } else {
        setAccessToken(undefined);
        apiClient?.setSecurityData(undefined);
        sessionStorage.removeItem(accessTokenKey);
      }
    },
    [apiClient],
  );

  const parseJwt = (token: string) => {
    try {
      if (!token) return null;
      const arrayToken = token.split('.');
      return JSON.parse(atob(arrayToken[1]));
    } catch (error) {
      console.error('Failed to parse JWT:', error);
      return null;
    }
  };

  // Handle access token updates from the API
  useEffect(() => {
    if (isSuccess && accessTokenData?.data.accessToken) {
      handleSetAccessToken(accessTokenData.data.accessToken);
    }
  }, [accessTokenData, isSuccess, handleSetAccessToken]);

  // Handle 401 errors by logging out
  useEffect(() => {
    if (accessTokenError && (accessTokenError as any)?.error?.status === 401) {
      logout({});
    }
  }, [accessTokenError, logout]);

  // Handle logout success
  useEffect(() => {
    if (logoutIsSuccess) {
      handleSetAccessToken(undefined);
      window.location.href = '/login';
    }
  }, [logoutIsSuccess, handleSetAccessToken]);

  // Automatically refresh the token before it expires
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

  return (
    <AuthContext.Provider
      value={{
        accessToken,
        isAuthenticated,
      }}>
      {children}
    </AuthContext.Provider>
  );
};
