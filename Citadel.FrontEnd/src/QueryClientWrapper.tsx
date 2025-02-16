import {
  DefaultError,
  MutationCache,
  Query,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query';
import { ProblemDetails } from './api/_generated';
import { createContext, useState } from 'react';
import { useRequiredContext } from './hooks/useRequiredContext';

interface IContext {
  error: ProblemDetails | undefined;
}

interface IProps {
  children?: React.ReactNode;
}

const QueryClientContext = createContext<IContext | undefined>(undefined);

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const [error, setError] = useState<ProblemDetails>();
  const onQueryError = (error: any, _query: Query<_, _, _>) => setError(error);
  const onMutationError = (error: DefaultError, _variables: any, _context: any, _mutations: any) =>
    setError(error as ProblemDetails);

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

  return (
    <QueryClientContext.Provider value={{ error }}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </QueryClientContext.Provider>
  );
};

export default QueryClientWrapper;

export const useQueryClientContext = () => useRequiredContext(QueryClientContext);
