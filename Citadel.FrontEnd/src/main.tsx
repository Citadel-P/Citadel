import React from 'react';
import { createRoot } from 'react-dom/client';
import { Toaster } from '@/components/ui/sonner';

import App from './App.tsx';
import './main.css';

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
    <Toaster richColors theme="light" toastOptions={{}} />
  </React.StrictMode>,
);
