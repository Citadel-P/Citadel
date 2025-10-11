import { useEffect, useState, useMemo, useCallback } from 'react';
import { useApiClientContext } from '@/api/ApiClientContext';
import { useHTTPErrorHandler } from './hooks/useHTTPErrorHandler';
import { useTokenRefresh } from './hooks/useTokenRefresh';
import { AuthContext } from './AuthContext';
import { toast } from 'sonner';
import { ProblemDetails } from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';

export const ACCESS_TOKEN_KEY = 'access_token';
const storedJwt = sessionStorage.getItem(ACCESS_TOKEN_KEY);

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { apiClient } = useApiClientContext();
  const { mutate: logout } = useMutate('logout');
  const [accessToken, setAccessToken] = useState<string | undefined>(storedJwt ?? undefined);
  const isAuthenticated = useMemo(() => accessToken != null, [accessToken]);
  const { data, isSuccess, error } = useTokenRefresh(accessToken, isAuthenticated);

  const handleSetAccessToken = useCallback(
    (token: string | undefined) => {
      if (token) {
        setAccessToken(token);
        apiClient?.setSecurityData(token);
        sessionStorage.setItem(ACCESS_TOKEN_KEY, token);
      } else {
        setAccessToken(undefined);
        apiClient?.setSecurityData(undefined);
        sessionStorage.removeItem(ACCESS_TOKEN_KEY);
      }
    },
    [apiClient],
  );

  useEffect(() => {
    if ((error as ProblemDetails)?.status === 401) {
      toast.error('Session expired', {
        description: 'Please login again',
      });
      setAccessToken(undefined);
      logout({});
    }
  }, [error, logout]);

  // Handle access token updates from the API
  useEffect(() => {
    if (isSuccess && data?.data.accessToken) {
      handleSetAccessToken(data.data.accessToken);
    }
  }, [data, isSuccess, handleSetAccessToken]);

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
