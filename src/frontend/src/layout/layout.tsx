import { Suspense } from 'react';
import Loader from '@/components/ui/loader';
import { ErrorBoundary } from '@/components/custom/error-boundary';
import { Outlet, useLocation } from 'react-router';
import { useLayoutContext } from '../lib/context/layout-context';
import { LayoutProvider } from '../lib/context/layout.provider';
import { AppProvider } from '@/lib/context/app-provider';
import { Sidebar } from './sidebar/sidebar';
import { UpdateNotice } from './update-notice';
import { LicenseReminder } from './license-reminder';
import { Header } from './header';
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar';
import { ActivityTaskSheet, AlertTaskSheet } from '@/features/alerters/alert-events/alert-task-sheet';

const LayoutPage = () => {
  const location = useLocation();
  const { sidebarMinimized, setSidebarOpen } = useLayoutContext();

  return (
    <SidebarProvider
      open={!sidebarMinimized}
      onOpenChange={setSidebarOpen}
      className={`${sidebarMinimized ? 'sidebar-collapsed' : 'sidebar-expanded'} h-dvh overflow-hidden`}>
      <Sidebar />
      <SidebarInset className="overflow-hidden bg-background">
        <Header />
        <AlertTaskSheet />
        <ActivityTaskSheet />
        <div id="main-scroll-container" className="min-h-0 grow overflow-auto bg-muted/15">
          <LicenseReminder />
          {location.pathname === '/' && <UpdateNotice />}
          <ErrorBoundary key={location.pathname}>
            <Suspense fallback={<Loader />}>
              <Outlet />
            </Suspense>
          </ErrorBoundary>
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
};

const Layout = () => {
  return (
    <AppProvider>
      <LayoutProvider>
        <LayoutPage />
      </LayoutProvider>
    </AppProvider>
  );
};

export default Layout;
