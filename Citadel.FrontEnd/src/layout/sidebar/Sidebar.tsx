import { useLayoutContext } from '@/layout/LayoutProvider';
import { ChevronsRight, ChevronsUpDown, Info } from 'lucide-react';
import LogoIcon from '@/assets/logo.svg';
import { SidebarMenu } from './SidebarMenu';
import { useNavigate } from 'react-router';
import useAnimation from '@/hooks/useAnimation';
import { SidebarDropDown } from './SidebarDropDown';

export const Sidebar = () => {
  const navigate = useNavigate();
  const { toggleSidebar, sidebarMinimized } = useLayoutContext();

  const { ref, open, setOpen } = useAnimation('dropDown');
  return (
    <aside
      className={`w-[calc(var(--sidebar-width)-40px)] scrollbar-thumb-rounded scrollbar-track-rounded hidden h-full flex-col justify-between bg-background pt-3 transition-all duration-300 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-card lg:flex`}>
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

      <div className="flex justify-between items-center mx-4 my-4 space-y-1 hover:bg-card">
        {/* Version */}
        <a
          target="_blank"
          rel="noreferrer"
          href="https://github.com/Citadel-P/Citadel"
          className="group flex h-9 cursor-pointer items-center justify-start rounded p-2">
          <Info width={18} className="text-muted-foreground/50" />

          {sidebarMinimized ? (
            <div className="fixed w-full">
              <span className="z-1 absolute left-12 -top-4 w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover:scale-100">
                v 1.0.0
              </span>
            </div>
          ) : (
            <div className="ml-3 truncate text-[10px] font-semibold tracking-wide focus:outline-hidden">
              <span className="rounded-lg bg-primary/10 px-2 font-semibold text-primary">v 1.0.0 </span>
            </div>
          )}
        </a>
        {!sidebarMinimized && (
          <div
            ref={ref}
            onClick={() => setOpen(!open)}
            className="mr-1 relative hover:cursor-pointer rounded-full hover:bg-foreground/5 p-1">
            <span>
              <ChevronsUpDown width={15} className="text-muted-foreground/90" />
            </span>
            {open && <SidebarDropDown />}
          </div>
        )}
      </div>
    </aside>
  );
};
