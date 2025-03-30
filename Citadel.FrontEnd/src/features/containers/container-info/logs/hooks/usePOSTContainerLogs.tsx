import { StreamLogsRequest } from '@/api/_generated';
import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';
import { AuthContext } from '@/features/auth/AuthProvider';
import { Cancellable } from '@/api/models';

export const usePOSTContainerLogs = (onChunkReceived: (chunk: string) => void) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const accessToken = useContextSelector(AuthContext, (s) => s?.accessToken);

  const mutationFn = async (param: StreamLogsRequest & Cancellable) => {
    if (!apiClient?.baseUrl) {
      throw new Error('API client base URL is not defined');
    }

    const response = await fetch(`${apiClient.baseUrl}/api/v1/containers/stream-logs`, {
      method: 'POST',
      credentials: 'include',
      signal: param.signal || null,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify(param),
    });

    if (!response.ok) {
      throw new Error(`Network response was not ok: ${response.status} ${response.statusText}`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('ReadableStream is not supported or response body is null');
    }

    const decoder = new TextDecoder('utf-8');
    let done = false;

    try {
      while (!done) {
        const { value, done: readerDone } = await reader.read();
        done = readerDone;

        if (value) {
          const chunk = decoder.decode(value, { stream: true });
          onChunkReceived(chunk);
        }
      }
    } catch (error) {
      console.error('Error while reading stream:', error);
      throw error;
    } finally {
      reader.releaseLock();
    }
  };

  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn,
  });

  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, validationErrors, error, isSuccess, data };
};
