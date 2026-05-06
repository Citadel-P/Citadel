import { useEffect, useCallback, useReducer } from 'react';
import { useApiClientContext } from '@/api/api-client-context';
import { AuthContext } from './auth-context';
import { useHTTPErrorHandler, useMutate } from '@/lib/hooks';
import { LoginRequest, ProblemDetails } from '@/api/generated/api.types';
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
  const { mutate: requestLogin, isPending, validationErrors } = useMutate('login');

  const [authState, dispatch] = useReducer(authReducer, initialState);

  const isAuthenticated = authState.status === 'authenticated';
  const isAuthReady = authState.status !== 'loading';

  const {
    token: refreshedToken,
    isSuccess,
    error,
    didAttemptRefresh,
    triggerManualRefresh,
  } = useTokenRefresh(authState.token, isAuthenticated);

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

  // Bootstrap (silent refresh)
  useEffect(() => {
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
  }, [triggerManualRefresh, applyToken]);

  // Successful automatic / manual refresh
  useEffect(() => {
    if (isSuccess && refreshedToken) {
      applyToken(refreshedToken);
    }
  }, [isSuccess, refreshedToken, applyToken]);

  // Handle expired cookie / refresh 401
  useEffect(() => {
    if ((error as ProblemDetails)?.status === 401 && didAttemptRefresh.current) {
      logout();
    }
  }, [error, logout, didAttemptRefresh]);

  return (
    <AuthContext.Provider
      value={{
        accessToken: authState.token,
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
