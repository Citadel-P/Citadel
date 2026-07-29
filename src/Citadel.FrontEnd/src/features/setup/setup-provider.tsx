import { useApiClientContext } from '@/api/api-client-context';
import { useCallback, useEffect, useReducer } from 'react';
import { SetupContext, SetupError } from './setup-context';

type SetupState =
  | { status: 'loading'; attempt: number }
  | { status: 'pending'; attempt: number }
  | { status: 'complete'; attempt: number }
  | { status: 'error'; attempt: number; error: SetupError };

type SetupAction =
  | { type: 'PENDING' }
  | { type: 'COMPLETE' }
  | { type: 'ERROR'; error: SetupError }
  | { type: 'RETRY' };

function reducer(state: SetupState, action: SetupAction): SetupState {
  switch (action.type) {
    case 'PENDING':
      return { status: 'pending', attempt: state.attempt };
    case 'COMPLETE':
      return { status: 'complete', attempt: state.attempt };
    case 'ERROR':
      return { status: 'error', attempt: state.attempt, error: action.error };
    case 'RETRY':
      return { status: 'loading', attempt: state.attempt + 1 };
  }
}

export function SetupProvider({ children }: { children: React.ReactNode }) {
  const { apiClient } = useApiClientContext();
  const [state, dispatch] = useReducer(reducer, { status: 'loading', attempt: 0 });

  useEffect(() => {
    if (state.status !== 'loading') return;

    const controller = new AbortController();
    apiClient.api
      .getSetupStatus({ signal: controller.signal })
      .then((response) => {
        if (controller.signal.aborted) return;
        dispatch({ type: response.data.requiresSetup ? 'PENDING' : 'COMPLETE' });
      })
      .catch((error: unknown) => {
        if (!controller.signal.aborted) {
          dispatch({
            type: 'ERROR',
            error: getSetupError(error),
          });
        }
      });

    return () => controller.abort();
  }, [apiClient, state.status, state.attempt]);

  const markSetupComplete = useCallback(() => dispatch({ type: 'COMPLETE' }), []);
  const retry = useCallback(() => dispatch({ type: 'RETRY' }), []);

  return (
    <SetupContext.Provider
      value={{
        isSetupReady: state.status !== 'loading',
        requiresSetup: state.status === 'pending',
        error: state.status === 'error' ? state.error : undefined,
        markSetupComplete,
        retry,
      }}>
      {children}
    </SetupContext.Provider>
  );
}

function getSetupError(error: unknown): SetupError {
  if (isHttpResponse(error)) {
    return {
      title: 'Citadel unavailable',
      message: 'Citadel could not load its startup status. Check the server logs and try again.',
    };
  }

  return {
    title: 'Cannot connect to Citadel',
    message: 'The Citadel server is not reachable. Check that it is running and try again.',
  };
}

function isHttpResponse(error: unknown): error is { status: number } {
  return typeof error === 'object' && error !== null && 'status' in error && typeof error.status === 'number';
}
