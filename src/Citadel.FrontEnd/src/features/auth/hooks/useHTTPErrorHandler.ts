import { useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { ProblemDetails } from '@/api/generated/api.types';
import { toast } from 'sonner';

export function useHTTPErrorHandler() {
  const client = useQueryClient();

  useEffect(() => {
    const handleError = (error: ProblemDetails) => {
      if (!error) return;
      if (error.status === 401) return; // handled by AuthProvider
      if (error.status != null && (error.status as number) > 400) {
        toast.error(error.status + ' ' + error.title, {
          description: error.detail,
        });
      }
    };
    const mutationUnsubscribe = client.getMutationCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error.error);
      }
    });

    const queryUnsubscribe = client.getQueryCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error.error);
      }
    });

    return () => {
      mutationUnsubscribe();
      queryUnsubscribe();
    };
  }, [client]);
}
