import { useEffect, useState, useCallback } from 'react';
import { useApiClientContext } from '@/api/api-client-context';
import { AuthContext } from './auth-context';
import { useHTTPErrorHandler, useMutate } from '@/lib/hooks';
import { LoginRequest, ProblemDetails } from '@/api/generated/api.types';
import { useTokenRefresh } from './hooks/use-token-refresh';

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { apiClient } = useApiClientContext();
  const { mutate: requestLogout } = useMutate('logout');
  const { mutate: requestLogin, isPending, validationErrors } = useMutate('login');

  const [accessToken, setAccessToken] = useState<string | undefined>(undefined);
  const [isAuthReady, setIsAuthReady] = useState(false);
  const isAuthenticated = Boolean(accessToken);

  const { token, isSuccess, error, didAttemptRefresh, triggerManualRefresh } = useTokenRefresh(
    accessToken,
    isAuthenticated,
  );

  const applyToken = useCallback(
    (token?: string) => {
      apiClient.setSecurityData(token);
      setAccessToken(token);
    },
    [apiClient],
  );

  const login = useCallback(
    (request: LoginRequest) => {
      requestLogin(request, {
        onSuccess: (data) => {
          if (data?.data?.accessToken) {
            applyToken(data.data.accessToken);
          }
        },
      });
    },
    [requestLogin, applyToken],
  );

  const logout = useCallback(() => {
    requestLogout({});
    applyToken(undefined);
  }, [requestLogout, applyToken]);

  /** Bootstrap: attempt silent refresh using cookie */
  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      try {
        const refreshed = await triggerManualRefresh();
        if (!cancelled && refreshed) {
          applyToken(refreshed);
        }
      } catch (err) {
        console.error('Bootstrap refresh failed:', err);
      } finally {
        if (!cancelled) setIsAuthReady(true);
      }
    };

    void init();
    return () => {
      cancelled = true;
    };
  }, [applyToken, triggerManualRefresh]);

  /** Apply new token when periodic refresh succeeds */
  useEffect(() => {
    if (isSuccess && token) {
      applyToken(token);
    }
  }, [isSuccess, token, applyToken]);

  /** Handle expired cookie / refresh 401 */
  useEffect(() => {
    if ((error as ProblemDetails)?.status === 401 && didAttemptRefresh.current) {
      logout();
    }
  }, [error, logout, didAttemptRefresh]);

  return (
    <AuthContext.Provider
      value={{
        accessToken,
        isAuthenticated,
        isAuthReady,
        login,
        logout,
        isPending,
        validationErrors,
      }}>
      {children}
    </AuthContext.Provider>
  );
};
