import {
  DefaultError,
  MutationCache,
  Query,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query';
import { ProblemDetails } from './api/_generated';
import { useAuthContext } from './features/login/AuthProvider';
import { toast } from 'sonner';

interface IProps {
  children?: React.ReactNode;
}

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const { logout } = useAuthContext();
  const onQueryError = (error: any, _query: Query<_, _, _>) => handleError(error as ProblemDetails);

  const onMutationError = (error: DefaultError, _variables: any, _context: any, _mutations: any) =>
    handleError(error as ProblemDetails);

  const handleError = (error: ProblemDetails): void => {
    if (error?.status === 401) {
      logout();
    }
    if (error?.status && error?.status >= 500) {
      const problem = error.error as ProblemDetails;
      toast.error(problem.status + ' ' + problem.title, {
        description: problem.detail,
      });
    }
  };

  const queryClient = new QueryClient({
    queryCache: new QueryCache({ onError: onQueryError }),
    mutationCache: new MutationCache({ onError: onMutationError }),
    defaultOptions: {
      queries: {
        retry: false,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
        refetchOnMount: true,
      },
      mutations: {
        retry: false,
      },
    },
  });

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
};

export default QueryClientWrapper;
