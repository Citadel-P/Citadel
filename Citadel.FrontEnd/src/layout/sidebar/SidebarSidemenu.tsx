import { Link } from 'react-router';
import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';
import { ISubMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';

interface IProps {
  submenu: ISubMenuItem;
  toggleMenu: (menu: ISubMenuItem) => void;
}

export const SidebarSubMenu = ({ submenu, toggleMenu }: IProps) => {
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized);

  return (
    <div
      className={`${sidebarMinimized ? 'hidden' : ''} ${submenu.expanded ? 'max-h-screen' : ''} max-h-0 overflow-hidden pt-1 pl-4 transition-all duration-500`}>
      <ul className="flex flex-col border-l border-dashed border-border pl-2 text-muted-foreground">
        {submenu.children?.map((sub, index) => (
          <li key={index}>
            <div
              className="flex rounded text-muted-foreground hover:bg-card hover:text-foreground"
              onClick={() => toggleMenu(sub)}>
              {sub.children ? (
                <>
                  <span className="inline-block w-full cursor-pointer px-4 py-2 text-xs font-semibold">
                    {sub.label}
                  </span>
                  <button
                    className={`${sidebarMinimized ? 'hidden' : ''} ${sub.expanded ? 'rotate-90' : ''} flex items-center p-1 text-muted-foreground transition-all duration-500`}>
                    <ChevronRight width={18} height={18} />
                  </button>
                </>
              ) : (
                <Link
                  to={sub.route ?? '/'}
                  className={`${sub.active ? 'text-primary' : ''} inline-block w-full px-4 py-2 text-xs font-semibold`}>
                  {sub.label}
                </Link>
              )}
            </div>
            <SidebarSubMenu submenu={sub} toggleMenu={toggleMenu} />
          </li>
        ))}
      </ul>
    </div>
  );
};
