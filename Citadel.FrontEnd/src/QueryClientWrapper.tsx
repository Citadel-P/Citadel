import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

interface IProps {
  children?: React.ReactNode;
}

const retryOnceOn401 = (failureCount: number, error: any): boolean => {
  // Only retry once (when failureCount is 0) for 401 errors
  if (error?.response?.status === 401) {
    return failureCount === 0;
  }

  // For other errors, use default retry logic
  return failureCount < 3;
};

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: retryOnceOn401,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
        refetchOnMount: true,
      },
      mutations: {
        retry: retryOnceOn401,
      },
    },
  });

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
};

export default QueryClientWrapper;
