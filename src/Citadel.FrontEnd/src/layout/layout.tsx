import { Outlet } from 'react-router';
import { useLayoutContext } from '../lib/context/layout-context';
import { LayoutProvider } from '../lib/context/layout.provider';
import { AppProvider } from '@/lib/context/app-provider';
import { useEffect, useRef, useState } from 'react';
import { Sidebar } from './sidebar/sidebar';
import Breadcrumb from './breadcrumb';
import { useRead } from '@/lib/hooks';
import { toThemeMode } from '@/lib/theme-preferences';

const LayoutPage = () => {
  const breadcrumbRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [isSticky, setIsSticky] = useState(false);
  const { sidebarMinimized, setThemeMode } = useLayoutContext();
  const preferencesQuery = useRead('getProfilePreferences');
  const preferredTheme = preferencesQuery.data?.data.theme;

  useEffect(() => {
    if (preferredTheme) {
      setThemeMode(toThemeMode(preferredTheme));
    }
  }, [preferredTheme, setThemeMode]);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        // Trigger sticky only when breadcrumb is outside the content area
        setIsSticky(entry.boundingClientRect.top < 0 && !entry.isIntersecting);
      },
      {
        root: contentRef.current, // Observe relative to the content area
        threshold: 0, // Trigger when the breadcrumb leaves the content area
      },
    );

    const breadcrumbElement = breadcrumbRef.current;
    if (breadcrumbElement) {
      observer.observe(breadcrumbElement);
    }

    return () => {
      if (breadcrumbElement) {
        observer.unobserve(breadcrumbElement);
      }
    };
  }, []);
  return (
    <div className={`${sidebarMinimized ? 'sidebar-collapsed' : 'sidebar-expanded'} flex h-dvh w-full overflow-hidden`}>
      <Sidebar />
      <main className="flex grow relative flex-col content-start overflow-hidden bg-card">
        <div id="main-scroll-container" ref={contentRef} className="grow overflow-auto">
          <span ref={breadcrumbRef}>
            <Breadcrumb isSticky={isSticky} />
          </span>
          <Outlet />
        </div>
      </main>
    </div>
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
