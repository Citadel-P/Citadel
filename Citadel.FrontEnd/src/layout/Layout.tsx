import { Outlet } from 'react-router';
import LayoutProvider, { LayoutContext } from './LayoutProvider';
import { Sidebar } from './sidebar/Sidebar';
import Breadcrumb from './breadcrumb/Breadcrumb';
import AppProvider from '@/AppProvider';
import { useContextSelector } from 'use-context-selector';
import { useEffect, useRef, useState } from 'react';

const LayoutPage = () => {
  const breadcrumbRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [isSticky, setIsSticky] = useState(false);
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized);

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

    if (breadcrumbRef.current) {
      observer.observe(breadcrumbRef.current);
    }

    return () => {
      if (breadcrumbRef.current) {
        observer.unobserve(breadcrumbRef.current);
      }
    };
  }, []);
  return (
    <div className={`${sidebarMinimized ? 'sidebar-collapsed' : 'sidebar-expanded'} flex h-dvh w-full overflow-hidden`}>
      <Sidebar />
      <main className="flex grow relative flex-col content-start overflow-hidden bg-card">
        <div
          ref={contentRef}
          className="scrollbar-thumb-rounded scrollbar-track-rounded grow overflow-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
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
