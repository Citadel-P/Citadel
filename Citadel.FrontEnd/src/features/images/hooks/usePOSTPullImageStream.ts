import { useApiClientContext } from '@/api/ApiClientProvider';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { PullImageRequest } from '@/api/_generated';
import { useAuthContext } from '@/features/auth/AuthProvider';
import { Cancellable } from '@/api/models';

export const usePOSTPullImageStream = (onChunkReceived: (chunk: string) => void) => {
  const { apiClient } = useApiClientContext();
  const { accessToken } = useAuthContext();

  const mutationFn = async (param: PullImageRequest & Cancellable) => {
    if (!apiClient?.baseUrl) {
      throw new Error('API client base URL is not defined');
    }

    const response = await fetch(`${apiClient.baseUrl}/api/v1/images/pull`, {
      method: 'POST',
      credentials: 'include',
      signal: param.signal,
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

  const { mutate, isPending, isSuccess, error } = useMutation({ mutationFn });

  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, error, validationErrors };
};
