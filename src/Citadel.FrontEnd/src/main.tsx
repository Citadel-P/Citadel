import React, { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { Toaster } from '@/components/ui/sonner';
import './main.css';
import QueryClientWrapper from './query-client-wrapper';
import { ApiClientProvider } from './api/api-client-provider';
import { AuthProvider } from './features/auth/auth-provider';
import LoadingBarWrapper from './components/custom/loading-bar-wrapper';
import { Router } from './router';
import { preloadMonaco } from './lib/monaco/monaco-preloader';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientWrapper>
      <ApiClientProvider>
        <AuthProvider>
          <LoadingBarWrapper />
          <AppInitializer />
          <Router />
          <Toaster richColors toastOptions={{}} />
        </AuthProvider>
      </ApiClientProvider>
    </QueryClientWrapper>
  </React.StrictMode>,
);

// Load monaco once Citadel is started
export function AppInitializer() {
  useEffect(() => {
    preloadMonaco();
  }, []);

  return null;
}
