import { useCallback, useEffect, useState } from 'react';
import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';
import { ISubMenuItem, MenuItems } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation, useNavigate } from 'react-router';
import { SidebarSubMenu } from './SidebarSidemenu';

export const SidebarMenu = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized) ?? false;
  const toggleSidebar = useContextSelector(LayoutContext, (v) => v?.toggleSidebar)!;
  const [menuItems, setMenuItems] = useState(MenuItems);

  const isRouteActive = useCallback(
    (path: string) => location.pathname === path || (location.pathname === '/' && path === ''),
    [location.pathname],
  );

  useEffect(() => {
    const updateMenuItems = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((item) => {
        const isActive = isRouteActive(item.route ?? '');
        return {
          ...item,
          active: isActive,
          expanded: item.children ? item.expanded || item.children.some((s) => s.expanded) : isActive,
          children: item.children ? updateMenuItems(item.children) : undefined,
        };
      });

    setMenuItems((v) =>
      v.map((menu) => ({
        ...menu,
        active: menu.items.some((subMenu) => isRouteActive(subMenu.route ?? '')),
        items: updateMenuItems(menu.items),
      })),
    );
  }, [location, isRouteActive]);

  const toggleMenu = (menu: ISubMenuItem): void => {
    const updateSubMenu = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((item) => {
        if (item === menu) {
          // Toggle the expanded state of the matched menu
          return { ...item, expanded: !item.expanded };
        }

        // Recursively update children if they exist
        if (item.children) {
          return { ...item, children: updateSubMenu(item.children) };
        }

        return item;
      });

    // Update MenuItems
    setMenuItems((prevMenuItems) =>
      prevMenuItems.map((menuGroup) => ({
        ...menuGroup,
        items: updateSubMenu(menuGroup.items),
      })),
    );

    // Handle sidebar toggle or navigation
    if (sidebarMinimized && menu.children) {
      toggleSidebar();
    } else if (!menu.children) {
      navigate(menu.route!);
    }
  };

  const renderMenuItem = (item: ISubMenuItem) => (
    <li key={item.label}>
      <div
        onClick={() => toggleMenu(item)}
        className="group relative text-muted-foreground cursor-pointer"
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') toggleMenu(item);
        }}>
        <div
          className={`${
            item.active && sidebarMinimized ? 'text-primary' : 'text-muted-foreground/50'
          } pointer-events-none absolute m-2`}>
          {item.icon}
        </div>

        {item.children ? (
          <ExpandableMenuItem item={item} sidebarMinimized={sidebarMinimized ?? false} />
        ) : (
          <div className="flex h-9 items-center justify-start rounded text-muted-foreground hover:bg-card hover:text-foreground">
            <Link
              to={item.route!}
              className={`${item.active ? 'text-primary' : ''} ml-10 truncate text-xs font-semibold tracking-wide`}>
              {item.label}
            </Link>
          </div>
        )}

        {/* Tooltip */}
        {sidebarMinimized && (
          <div className="absolute w-full">
            <span className="z-100 absolute left-14 -top-[34px] w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover:scale-100">
              {item.label}
            </span>
          </div>
        )}
      </div>

      {/* Submenu items */}
      {item.children && <SidebarSubMenu submenu={item} toggleMenu={toggleMenu} />}
    </li>
  );

  interface ExpandableMenuItemProps {
    item: ISubMenuItem;
    sidebarMinimized: boolean;
  }

  const ExpandableMenuItem = ({ item, sidebarMinimized }: ExpandableMenuItemProps) => (
    <div className="flex h-9 items-center justify-start rounded hover:bg-card">
      <span className="ml-10 truncate text-xs font-semibold tracking-wide text-muted-foreground group-hover:text-foreground">
        {item.label}
      </span>
      <button
        className={`${
          sidebarMinimized ? 'hidden' : ''
        } ${item.expanded ? 'rotate-90' : ''} absolute top-1 right-0 flex items-center p-1 text-muted-foreground/50 transition-all transform duration-500`}
        aria-label="Expand submenu">
        <ChevronRight className="w-4 h-4" />
      </button>
    </div>
  );

  return (
    <>
      {menuItems.map((menu, i) => (
        <div className="pt-4" key={menu.group || i}>
          <div className="mx-1 mb-2 flex items-center justify-between">
            <small className={`${sidebarMinimized ? 'hidden' : ''} text-xs font-semibold text-muted-foreground/50`}>
              {menu.group}
            </small>
          </div>

          <ul className="flex flex-col space-y-1">{menu.items.map(renderMenuItem)}</ul>

          {menu.separator && (
            <div className="pt-3">
              <hr className="border-dashed border-border" />
            </div>
          )}
        </div>
      ))}
    </>
  );
};
