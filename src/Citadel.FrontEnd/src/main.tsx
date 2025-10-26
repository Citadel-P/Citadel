import React from 'react';
import { createRoot } from 'react-dom/client';
import { Toaster } from '@/components/ui/sonner';
import './main.css';
import { ErrorBoundary } from 'react-error-boundary';
import QueryClientWrapper from './query-client-wrapper';
import { ApiClientProvider } from './api/ApiClientProvider';
import { AuthProvider } from './features/auth/AuthProvider';
import LoadingBarWrapper from './components/custom/loading-bar-wrapper';
import { AppRoutes } from './app-routes';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
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
    <Toaster richColors toastOptions={{}} />
  </React.StrictMode>,
);

function Fallback({ error }: any) {
  return (
    <div>
      <p>Something went wrong:</p>
      <pre style={{ color: 'red' }}>{error.message}</pre>
    </div>
  );
}
