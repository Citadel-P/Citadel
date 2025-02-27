import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

interface IProps {
  children?: React.ReactNode;
}
const retryOnceOn401 = (failureCount: number, error: any): boolean => {
  if (error.response?.status !== 401) {
    return false;
  }
  return failureCount >= 1;
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
