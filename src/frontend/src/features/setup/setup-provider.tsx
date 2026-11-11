import { useApiClientContext } from '@/api/api-client-context';
import { resourceQueryKey } from '@/api/dispatch';
import { resourceReadOptions } from '@/api/read-options';
import type { ResourceResponse } from '@/api/types';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { SetupContext, SetupError } from './setup-context';
import { MfaPolicy } from '@/api/generated/api.types';

const setupQueryKey = resourceQueryKey('getSetupStatus');

export function SetupProvider({ children }: { children: React.ReactNode }) {
  const { apiClient } = useApiClientContext();
  const client = useQueryClient();
  const [setupCompleted, setSetupCompleted] = useState(false);
  const {
    data: response,
    error: requestError,
    isPending,
    isFetching,
    refetch,
  } = useQuery({
    ...resourceReadOptions(apiClient, 'getSetupStatus'),
    staleTime: Infinity,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
  const status = response?.data;
  // Reconnect refreshes the running policy through the shared cache. Keep the
  // established startup gate during a background refresh or transient failure.
  const error = useMemo(
    () => (!status && requestError ? getSetupError(requestError) : undefined),
    [status, requestError],
  );
  const markSetupComplete = useCallback(() => {
    // Route guards must see completion in the same render as navigation.
    setSetupCompleted(true);
    void client.cancelQueries({ queryKey: setupQueryKey });
    client.setQueryData<ResourceResponse<'getSetupStatus'>>(setupQueryKey, (current) =>
      current ? { ...current, data: { ...current.data, requiresSetup: false } } : current,
    );
  }, [client]);
  const retry = useCallback(() => {
    void refetch();
  }, [refetch]);

  return (
    <SetupContext.Provider
      value={{
        passwordMinimumLength: status?.passwordMinimumLength ?? 15,
        passwordMaximumLength: status?.passwordMaximumLength ?? 128,
        updateChecksEnabled: status?.updateChecksEnabled ?? true,
        updateChecksAllowed: status?.updateChecksAllowed ?? true,
        mfaPolicy: status?.mfaPolicy ?? MfaPolicy.Optional,
        minimumMfaPolicy: status?.minimumMfaPolicy ?? MfaPolicy.Optional,
        isSetupReady: !!status || (!isPending && !isFetching),
        requiresSetup: !setupCompleted && (status?.requiresSetup ?? false),
        error,
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
