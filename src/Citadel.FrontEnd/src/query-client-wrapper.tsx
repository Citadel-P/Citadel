import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';

interface IProps {
  children?: React.ReactNode;
}

// do not retry on http errors
const handleRetry = (): boolean => false;

export const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: handleRetry,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
        refetchOnMount: true,
      },
      mutations: {
        retry: handleRetry,
      },
    },
  });

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const [queryClient] = useState(createQueryClient);

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
};

export default QueryClientWrapper;
