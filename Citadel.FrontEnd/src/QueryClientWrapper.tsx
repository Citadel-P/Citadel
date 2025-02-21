import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

interface IProps {
  children?: React.ReactNode;
}

const QueryClientWrapper: React.FC<IProps> = ({ children }) => {
  const queryClient = new QueryClient({
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
