import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

interface IProps {
  children?: React.ReactNode;
}

// do not retry on http errors
const handleRetry = (): boolean => false;

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const queryClient = new QueryClient({
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

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
};

export default QueryClientWrapper;
