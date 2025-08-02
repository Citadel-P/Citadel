import { Link } from 'react-router';
import { useLayoutContext } from '@/layout/LayoutContext';
import { ISubMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';

interface IProps {
  submenu: ISubMenuItem;
  toggleMenu: (menu: ISubMenuItem) => void;
}

export const SidebarSubMenu = ({ submenu, toggleMenu }: IProps) => {
  const { sidebarMinimized } = useLayoutContext();
  return (
    <div
      className={`transition-all duration-500 overflow-hidden pt-1 pl-4 ${
        sidebarMinimized ? 'hidden' : submenu.expanded ? 'max-h-screen' : 'max-h-0'
      }`}>
      <ul className="flex flex-col border-l border-dashed border-border pl-2 text-muted-foreground">
        {submenu.children?.map((sub, index) => (
          <li key={index}>
            <div
              className="flex items-center rounded text-muted-foreground hover:bg-card hover:text-foreground"
              role="button"
              tabIndex={0}
              onClick={() => toggleMenu(sub)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  toggleMenu(sub);
                }
              }}>
              {sub.children ? (
                <ExpandableMenuItem sub={sub} sidebarMinimized={sidebarMinimized ?? false} />
              ) : (
                <Link
                  to={sub.route ?? '/'}
                  className={`inline-block w-full px-4 py-2 text-xs font-semibold ${sub.active ? 'text-primary' : ''}`}>
                  {sub.label}
                </Link>
              )}
            </div>
            {sub.children && sub.expanded && <SidebarSubMenu submenu={sub} toggleMenu={toggleMenu} />}
          </li>
        ))}
      </ul>
    </div>
  );
};

interface ExpandableMenuItemProps {
  sub: ISubMenuItem;
  sidebarMinimized: boolean;
}

const ExpandableMenuItem = ({ sub, sidebarMinimized }: ExpandableMenuItemProps) => (
  <div className="flex items-center justify-between w-full">
    <span className="inline-block cursor-pointer px-4 py-2 text-xs font-semibold">{sub.label}</span>
    <button
      className={`flex items-center p-1 text-muted-foreground transition-transform duration-500 ${
        sidebarMinimized ? 'hidden' : sub.expanded ? 'rotate-90' : ''
      }`}
      aria-label="Expand submenu">
      <ChevronRight />
    </button>
  </div>
);
