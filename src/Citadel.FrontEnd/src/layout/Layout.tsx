import { Outlet } from 'react-router';
import { useLayoutContext } from './LayoutContext';
import { LayoutProvider } from './LayoutProvider';
import { Sidebar } from './sidebar/Sidebar';
import Breadcrumb from './breadcrumb/Breadcrumb';
import { AppProvider } from '@/lib/context/app-provider';
import { useEffect, useRef, useState } from 'react';

const LayoutPage = () => {
  const breadcrumbRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [isSticky, setIsSticky] = useState(false);
  const { sidebarMinimized } = useLayoutContext();

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
        <div
          ref={contentRef}
          className="scrollbar-thumb-rounded scrollbar-track-rounded grow overflow-auto scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted dark:scrollbar-thumb-muted-foreground">
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
