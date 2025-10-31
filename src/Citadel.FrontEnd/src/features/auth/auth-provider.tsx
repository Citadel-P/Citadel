import { useEffect, useState, useCallback } from 'react';
import { useApiClientContext } from '@/api/api-client-context';
import { AuthContext } from './auth-context';
import { toast } from 'sonner';
import { ProblemDetails } from '@/api/generated/api.types';
import { useHTTPErrorHandler, useMutate } from '@/lib/hooks';
import { useTokenRefresh } from './hooks/use-token-refresh';

export const ACCESS_TOKEN_KEY = 'access_token';

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { apiClient } = useApiClientContext();
  const { mutate: requestLogout } = useMutate('logout');

  const [accessToken, setAccessToken] = useState<string | undefined>(
    () => sessionStorage.getItem(ACCESS_TOKEN_KEY) ?? undefined,
  );
  const [isAuthReady, setIsAuthReady] = useState(false);
  const isAuthenticated = Boolean(accessToken);

  const { token, isSuccess, error, didAttemptRefresh } = useTokenRefresh(accessToken, isAuthenticated);

  const applyToken = useCallback(
    (token?: string) => {
      if (token) {
        apiClient.setSecurityData(token);
        sessionStorage.setItem(ACCESS_TOKEN_KEY, token);
      } else {
        apiClient.setSecurityData(undefined);
        sessionStorage.removeItem(ACCESS_TOKEN_KEY);
      }
      setAccessToken(token);
    },
    [apiClient],
  );

  const logout = useCallback(() => {
    requestLogout({});
    applyToken(undefined);
  }, [requestLogout, applyToken]);

  // Restore token on mount
  useEffect(() => {
    const storedToken = sessionStorage.getItem(ACCESS_TOKEN_KEY);
    if (storedToken) apiClient.setSecurityData(storedToken);
    setIsAuthReady(true);
  }, [apiClient]);

  // Handle token refresh success
  useEffect(() => {
    if (isSuccess && token) {
      applyToken(token);
    }
  }, [isSuccess, token, applyToken]);

  // Handle session expiration
  useEffect(() => {
    if ((error as ProblemDetails)?.status === 401 && didAttemptRefresh.current) {
      toast.error('Session expired', { description: 'Please login again' });
      logout();
    }
  }, [error, logout, didAttemptRefresh]);

  return (
    <AuthContext.Provider
      value={{
        accessToken,
        isAuthenticated,
        isAuthReady,
        logout,
        setAccessToken: applyToken,
      }}>
      {children}
    </AuthContext.Provider>
  );
};
