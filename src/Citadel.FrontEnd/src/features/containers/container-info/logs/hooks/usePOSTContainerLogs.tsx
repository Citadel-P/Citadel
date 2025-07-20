import { StreamLogsRequest } from '@/api/_generated';
import { useApiClientContext } from '@/api/ApiClientProvider';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { useMutation } from '@tanstack/react-query';
import { useAuthContext } from '@/features/auth/AuthProvider';
import { Cancellable } from '@/api/models';

export const usePOSTContainerLogs = (onChunkReceived: (chunk: string) => void) => {
  const { apiClient } = useApiClientContext();
  const { accessToken } = useAuthContext();
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
    let buffer = '';

    try {
      while (!done) {
        const { value, done: readerDone } = await reader.read();
        done = readerDone;

        if (value) {
          buffer += decoder.decode(value, { stream: true });

          let newlineIndex;
          while ((newlineIndex = buffer.indexOf('\n')) >= 0) {
            const line = buffer.slice(0, newlineIndex).trim();
            buffer = buffer.slice(newlineIndex + 1); // remove the processed line

            if (line) {
              try {
                const parsed = JSON.parse(line);
                onChunkReceived(parsed.Log);
              } catch {
                console.warn('Failed to parse JSON line:', line);
              }
            }
          }
        }
      }

      // Final flush (in case last line has no newline)
      if (buffer.trim()) {
        try {
          const parsed = JSON.parse(buffer.trim());
          onChunkReceived(parsed.log);
        } catch {
          console.warn('Failed to parse final JSON chunk:', buffer);
        }
      }
    } catch (error) {
      console.error('Stream error:', error);
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
