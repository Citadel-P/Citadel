import { ErrorBoundary } from 'react-error-boundary';
import { AppRoutes } from './AppRoutes';
import ApiClientProvider from './api/ApiClientProvider';
import AuthProvider from './features/login/AuthProvider';
import QueryClientWrapper from './QueryClientWrapper';
import LoadingBarWrapper from './LoadingBarWrapper';

function App() {
  const classNames = ['bg-background', 'font-poppins', 'selection:bg-primary', 'selection:text-primary-foreground'];
  document.body.classList.add(...classNames);

  return (
    <ErrorBoundary FallbackComponent={Fallback}>
      <AuthProvider>
        <QueryClientWrapper>
          <ApiClientProvider>
            <LoadingBarWrapper />
            <AppRoutes />
          </ApiClientProvider>
        </QueryClientWrapper>
      </AuthProvider>
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
