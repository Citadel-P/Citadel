import React from 'react';
import { createRoot } from 'react-dom/client';
import { Toaster } from '@/components/ui/sonner';
import './main.css';
import { ErrorBoundary } from 'react-error-boundary';
import QueryClientWrapper from './query-client-wrapper';
import { ApiClientProvider } from './api/api-client-provider';
import { AuthProvider } from './features/auth/auth-provider';
import LoadingBarWrapper from './components/custom/loading-bar-wrapper';
import { Router } from './router';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary FallbackComponent={Fallback}>
      <QueryClientWrapper>
        <ApiClientProvider>
          <AuthProvider>
            <LoadingBarWrapper />
            <Router />
            <Toaster richColors toastOptions={{}} />
          </AuthProvider>
        </ApiClientProvider>
      </QueryClientWrapper>
    </ErrorBoundary>
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
