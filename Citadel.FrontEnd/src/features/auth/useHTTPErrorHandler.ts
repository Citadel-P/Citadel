import { useQueryClient } from '@tanstack/react-query';
import { usePOSTLogout } from './usePOSTLogout';
import { useEffect } from 'react';
import { ProblemDetails } from '@/api/_generated';
import { toast } from 'sonner';

export function useHTTPErrorHandler() {
  const { mutate: logout } = usePOSTLogout();
  const client = useQueryClient();

  useEffect(() => {
    const handleError = (error: ProblemDetails) => {
      if (error?.status === 401) {
        logout({});
        window.location.href = '/login';
      } else if (error?.status != null && error?.status >= 400) {
        const problem = error.error as ProblemDetails;
        toast.error(problem.status + ' ' + problem.title, {
          description: problem.detail,
        });
      }
    };
    const mutationUnsubscribe = client.getMutationCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error);
      }
    });

    const queryUnsubscribe = client.getQueryCache().subscribe((event) => {
      if (event.type === 'updated' && event.action.type === 'error') {
        handleError(event.action.error);
      }
    });

    return () => {
      mutationUnsubscribe();
      queryUnsubscribe();
    };
  }, [client]);
}
