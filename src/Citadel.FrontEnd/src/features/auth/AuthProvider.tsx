import { useEffect, useState, useMemo, useCallback } from 'react';
import { useGETAccessToken } from './hooks/useGETAccessToken';
import { useApiClientContext } from '@/api/ApiClientContext';
import { useHTTPErrorHandler } from './hooks/useHTTPErrorHandler';
import { usePOSTLogout } from './hooks/usePOSTLogout';
import { useTokenRefresh } from './hooks/useTokenRefresh';
import { AuthContext } from './AuthContext';

const accessTokenKey = 'access_token';
const storedJwt = sessionStorage.getItem(accessTokenKey);

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { apiClient } = useApiClientContext();
  const { data, isSuccess, error } = useGETAccessToken();
  const { mutate: logout } = usePOSTLogout();
  const [accessToken, setAccessToken] = useState<string | undefined>(storedJwt ?? undefined);
  const isAuthenticated = useMemo(() => accessToken != null, [accessToken]);
  useTokenRefresh(accessToken, isAuthenticated);

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

  // Handle access token updates from the API
  useEffect(() => {
    if (isSuccess && data?.data.accessToken) {
      handleSetAccessToken(data.data.accessToken);
    }
  }, [data, isSuccess, handleSetAccessToken]);

  // Handle 401 errors by logging out
  useEffect(() => {
    if (error && (error as any)?.error?.status === 401) {
      logout({});
    }
  }, [error, logout]);

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
