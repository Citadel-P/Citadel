import { useEffect, useCallback, useReducer } from 'react';
import { useApiClientContext } from '@/api/api-client-context';
import { AuthContext } from './auth-context';
import { useHTTPErrorHandler, useMutate } from '@/lib/hooks';
import { LoginNextStep, LoginRequest, LoginResponse, ProblemDetails } from '@/api/generated/api.types';
import { useTokenRefresh } from './hooks/use-token-refresh';

type AuthState = {
  status: 'loading' | 'authenticated' | 'unauthenticated';
  token?: string;
};

type AuthAction = { type: 'SET_TOKEN'; token: string } | { type: 'LOGOUT' } | { type: 'INIT_FAILED' };

const initialState: AuthState = { status: 'loading' };

function authReducer(state: AuthState, action: AuthAction): AuthState {
  switch (action.type) {
    case 'SET_TOKEN':
      return { status: 'authenticated', token: action.token };
    case 'LOGOUT':
      return { status: 'unauthenticated', token: undefined };
    case 'INIT_FAILED':
      // Keep any token if we already had one? No – bootstrap failed, so unauthenticated.
      return { status: 'unauthenticated', token: undefined };
    default:
      return state;
  }
}

export const AuthProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  useHTTPErrorHandler();
  const { apiClient } = useApiClientContext();
  const { mutate: requestLogout } = useMutate('logout');
  const { mutateAsync: requestLogin, isPending, validationErrors } = useMutate('login');

  const [authState, dispatch] = useReducer(authReducer, initialState);

  const isAuthenticated = authState.status === 'authenticated';
  const isAuthReady = authState.status !== 'loading';

  const applyToken = useCallback(
    (token?: string) => {
      apiClient.setSecurityData(token);
      if (token) {
        dispatch({ type: 'SET_TOKEN', token });
      } else {
        dispatch({ type: 'LOGOUT' });
      }
    },
    [apiClient],
  );

  const requestRefreshToken = useCallback(async () => {
    const response = await apiClient.api.refreshToken({});
    return response.data.accessToken;
  }, [apiClient]);

  const { error, didAttemptRefresh, triggerManualRefresh, isAccessTokenExpired } = useTokenRefresh(
    authState.token,
    isAuthenticated,
    requestRefreshToken,
    applyToken,
  );

  const login = useCallback(
    async (request: LoginRequest): Promise<LoginResponse | undefined> => {
      const response = await requestLogin({ data: request });
      if (response.data.nextStep === LoginNextStep.Completed && response.data.accessToken) {
        applyToken(response.data.accessToken);
      }
      return response.data;
    },
    [requestLogin, applyToken],
  );

  const logout = useCallback(() => {
    requestLogout({});
    applyToken(undefined);
  }, [requestLogout, applyToken]);

  // Bootstrap (silent refresh)
  useEffect(() => {
    if (authState.status !== 'loading') return;

    const controller = new AbortController();
    const signal = controller.signal;

    const init = async () => {
      try {
        const newToken = await triggerManualRefresh();
        if (!signal.aborted) {
          if (newToken) {
            applyToken(newToken);
          } else {
            dispatch({ type: 'INIT_FAILED' });
          }
        }
      } catch (err) {
        console.error('Bootstrap refresh failed:', err);
        if (!signal.aborted) {
          dispatch({ type: 'INIT_FAILED' });
        }
      }
    };

    void init();
    return () => controller.abort();
  }, [authState.status, triggerManualRefresh, applyToken]);

  // Handle expired cookie / refresh 401
  useEffect(() => {
    const problem = ((error as { error?: ProblemDetails } | undefined)?.error ?? error) as ProblemDetails | undefined;
    if (problem?.status === 401 && didAttemptRefresh.current && (!authState.token || isAccessTokenExpired)) {
      logout();
    }
  }, [authState.token, error, isAccessTokenExpired, logout, didAttemptRefresh]);

  return (
    <AuthContext.Provider
      value={{
        accessToken: authState.token,
        isAuthenticated,
        isAuthReady,
        login,
        completeLogin: (accessToken) => applyToken(accessToken),
        logout,
        isPending,
        validationErrors,
      }}>
      {children}
    </AuthContext.Provider>
  );
};
