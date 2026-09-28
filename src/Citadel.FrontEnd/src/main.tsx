import React, { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { Toaster } from '@/components/ui/sonner';
import '@fontsource-variable/geist/index.css';
import '@fontsource-variable/inter/index.css';
import '@fontsource-variable/ibm-plex-sans/index.css';
import '@fontsource-variable/source-sans-3/index.css';
import { AppearanceProvider } from './lib/appearance/appearance-provider';
import { applyAppearanceToDocument } from './lib/appearance/apply-appearance';
import { readBootstrapAppearance } from './lib/appearance/appearance-storage';
import { FONT_OPTIONS } from './lib/appearance/appearance-config';
import './main.css';
import QueryClientWrapper from './query-client-wrapper';
import { ApiClientProvider } from './api/api-client-provider';
import { AuthProvider } from './features/auth/auth-provider';
import LoadingBarWrapper from './components/custom/loading-bar-wrapper';
import { Router } from './router';
import { preloadMonaco } from './lib/monaco/monaco-preloader';
import { SetupProvider } from './features/setup/setup-provider';
import { useSetupContext } from './features/setup/setup-context';

const initialAppearance = readBootstrapAppearance();
applyAppearanceToDocument(initialAppearance);
const initialFont = FONT_OPTIONS.find((option) => option.value === initialAppearance.font)!;
// Load only the selected local font before rendering text.
void document.fonts
  .load(`14px "${initialFont.family}"`)
  .catch(() => [])
  .then(() => {
    createRoot(document.getElementById('root')!).render(
      <React.StrictMode>
        <QueryClientWrapper>
          <ApiClientProvider>
            <SetupProvider>
              <AuthProvider>
                <AppearanceProvider>
                  <LoadingBarWrapper />
                  <AppInitializer />
                  <Router />
                  <Toaster richColors toastOptions={{}} />
                </AppearanceProvider>
              </AuthProvider>
            </SetupProvider>
          </ApiClientProvider>
        </QueryClientWrapper>
      </React.StrictMode>,
    );
  });

// Load monaco once Citadel is started
function AppInitializer() {
  const { isSetupReady, requiresSetup, error } = useSetupContext();

  useEffect(() => {
    if (!isSetupReady || requiresSetup || error) return;
    preloadMonaco();
  }, [isSetupReady, requiresSetup, error]);

  return null;
}
