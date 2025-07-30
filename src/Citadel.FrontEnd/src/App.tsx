import { ErrorBoundary } from 'react-error-boundary';
import { AppRoutes } from './AppRoutes';
import QueryClientWrapper from './QueryClientWrapper';
import { ApiClientProvider } from './api/ApiClientProvider';
import LoadingBarWrapper from './LoadingBarWrapper';
import { AuthProvider } from './features/auth/AuthProvider';

function App() {
  const classNames = ['bg-background', 'selection:bg-primary', 'selection:text-primary-foreground'];
  document.body.classList.add(...classNames);

  return (
    <ErrorBoundary FallbackComponent={Fallback}>
      <QueryClientWrapper>
        <ApiClientProvider>
          <AuthProvider>
            <LoadingBarWrapper />
            <AppRoutes />
          </AuthProvider>
        </ApiClientProvider>
      </QueryClientWrapper>
    </ErrorBoundary>
  );
}

function Fallback({ error }: any) {
  return (
    <div>
      <p>Something went wrong:</p>
      <pre style={{ color: 'red' }}>{error.message}</pre>
    </div>
  );
}

export default App;
