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
import { SetupProvider } from './features/setup/setup-provider';
import { useSetupContext } from './features/setup/setup-context';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientWrapper>
      <ApiClientProvider>
        <SetupProvider>
          <AuthProvider>
            <LoadingBarWrapper />
            <AppInitializer />
            <Router />
            <Toaster richColors toastOptions={{}} />
          </AuthProvider>
        </SetupProvider>
      </ApiClientProvider>
    </QueryClientWrapper>
  </React.StrictMode>,
);

// Load monaco once Citadel is started
function AppInitializer() {
  const { isSetupReady, requiresSetup, error } = useSetupContext();

  useEffect(() => {
    if (!isSetupReady || requiresSetup || error) return;
    preloadMonaco();
  }, [isSetupReady, requiresSetup, error]);

  return null;
}
