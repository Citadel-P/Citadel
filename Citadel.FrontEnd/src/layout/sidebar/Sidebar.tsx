import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';
import { ChevronsRight, Info } from 'lucide-react';
import LogoIcon from '@/assets/logo.svg';
import { SidebarMenu } from './SidebarMenu';
import { useNavigate } from 'react-router';

export const Sidebar = () => {
  const navigate = useNavigate();
  const toggleSidebar = useContextSelector(LayoutContext, (v) => v?.toggleSidebar);
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized);

  return (
    <nav
      className={`${sidebarMinimized ? 'w-[70px]' : 'w-52 xl:w-64'} scrollbar-thumb-rounded scrollbar-track-rounded hidden h-full flex-col justify-between overflow-auto bg-background pt-3 transition-all duration-300 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-card lg:flex`}>
      <div className="px-4">
        {/* Logo */}
        <div className="relative h-10">
          {!sidebarMinimized && (
            <div className="flex items-center">
              <span
                onClick={() => navigate('/')}
                className="flex text-background cursor-pointer items-center justify-center rounded bg-primary p-2 focus:outline-hidden focus:ring-1">
                <LogoIcon />
              </span>
              <b className="ml-1 pl-2 text-sm font-bold text-foreground"> Citadel </b>
            </div>
          )}
          <button
            onClick={toggleSidebar}
            className={`${sidebarMinimized ? '' : '-rotate-180'} absolute top-2 right-2 flex h-5 w-5 items-center justify-center rounded text-muted-foreground/50 transition-all duration-200 focus:outline-hidden hover:text-muted-foreground`}>
            <ChevronsRight />
          </button>
        </div>

        {/** Separator */}
        <div className="pt-3">
          <hr className="border-dashed border-muted" />
        </div>

        {/* Menu Items */}
        <SidebarMenu />
      </div>

      <div className="mx-4 my-4 space-y-1">
        {/* Version */}
        <a
          target="_blank"
          rel="noreferrer"
          href="https://github.com/Citadel-P/Citadel"
          className="group flex h-9 cursor-pointer items-center justify-start rounded p-2 hover:bg-card">
          <span className="h-6 w-5 text-muted-foreground/50">
            <Info width={18} />
          </span>

          <div className="ml-3 truncate text-[10px] font-semibold tracking-wide focus:outline-hidden">
            <span className="rounded-lg bg-primary/10 px-2 font-semibold text-primary">v 1.0.0 </span>
          </div>
          {sidebarMinimized && (
            <div className="fixed w-full">
              <span className="z-1 absolute left-12 -top-4 w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover:scale-100">
                v 1.0.0
              </span>
            </div>
          )}
        </a>
      </div>
    </nav>
  );
};
