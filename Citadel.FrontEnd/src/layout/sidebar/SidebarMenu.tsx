import { useCallback, useEffect, useState, useRef } from 'react';
import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';
import { ISubMenuItem, MenuItems, DockerPlatformMenu, IMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation, useNavigate } from 'react-router';
import { SidebarSubMenu } from './SidebarSidemenu';
import { AppContext } from '@/AppProvider';

export const SidebarMenu = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const sidebarMinimized = useContextSelector(LayoutContext, (v) => v?.sidebarMinimized) ?? false;
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const toggleSidebar = useContextSelector(LayoutContext, (v) => v?.toggleSidebar)!;
  const [menuItems, setMenuItems] = useState<IMenuItem[]>(MenuItems);
  const addedPlatformIdsRef = useRef<Set<string>>(new Set());

  const isRouteActive = useCallback(
    (path: string) => location.pathname === path || (location.pathname === '/' && path === ''),
    [location.pathname],
  );

  // Add platform menu dynamically
  const addPlatformToMenu = useCallback(
    (platform: { id: string; name: string }) => {
      if (!platform?.id) return;
      if (addedPlatformIdsRef.current.has(platform.id)) return;

      const platformRoute = `/platforms/${platform.id}`;
      const platformMenu = DockerPlatformMenu(platform);

      // Mark children as active/expanded if applicable
      platformMenu.children?.forEach((item) => {
        item.active = isRouteActive(item.route ?? '');
      });
      platformMenu.expanded = platformMenu.children?.some((menu) => menu.active) || false;
      addedPlatformIdsRef.current.add(platform.id);

      setMenuItems((prevMenuItems) => {
        const baseMenuIndex = prevMenuItems.findIndex((menu) => menu.group === 'Base');

        if (baseMenuIndex === -1) return prevMenuItems;

        const baseMenu = prevMenuItems[baseMenuIndex];

        // Double-check for duplicates in state (safety check)
        if (baseMenu.items.some((item) => item.route === platformRoute)) return prevMenuItems;

        // Add the new platform
        const updatedBaseMenu = {
          ...baseMenu,
          items: [...baseMenu.items, platformMenu],
        };

        return [...prevMenuItems.slice(0, baseMenuIndex), updatedBaseMenu, ...prevMenuItems.slice(baseMenuIndex + 1)];
      });
    },
    [isRouteActive],
  );

  // Clean up removed platforms
  useEffect(() => {
    const baseMenu = menuItems.find((menu) => menu.group === 'Base');
    if (baseMenu) {
      const currentPlatformIds = new Set<string>();

      // Extract platform IDs from current menu items
      baseMenu.items.forEach((item) => {
        if (item.isPlatform && item.route) {
          // Extract ID from route like "/platforms/123"
          const match = item.route.match(/\/platforms\/(.+)/);
          if (match && match[1]) {
            currentPlatformIds.add(match[1]);
          }
        }
      });

      // Update our ref to match the current state
      addedPlatformIdsRef.current = currentPlatformIds;
    }
  }, [menuItems]);

  // Update menu items when location changes
  useEffect(() => {
    const updateMenuItems = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((item) => ({
        ...item,
        active: isRouteActive(item.route ?? ''),
        expanded: item.children
          ? item.expanded || item.children.some((child) => child.expanded)
          : isRouteActive(item.route ?? ''),
        children: item.children ? updateMenuItems(item.children) : undefined,
      }));

    setMenuItems((prevMenuItems) =>
      prevMenuItems.map((menu) => ({
        ...menu,
        active: menu.items.some((subMenu) => isRouteActive(subMenu.route ?? '')),
        items: updateMenuItems(menu.items),
      })),
    );
  }, [location, isRouteActive]);

  // Add platform menu when currentPlatform changes
  useEffect(() => {
    if (currentPlatform?.id) {
      addPlatformToMenu({ id: currentPlatform.id, name: currentPlatform.name ?? '' });
    }
  }, [currentPlatform, addPlatformToMenu]);

  const toggleMenu = (menu: ISubMenuItem): void => {
    const updateSubMenu = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((item) => ({
        ...item,
        expanded: item === menu ? !item.expanded : item.expanded,
        children: item.children ? updateSubMenu(item.children) : undefined,
      }));

    setMenuItems((prevMenuItems) =>
      prevMenuItems.map((menuGroup) => ({
        ...menuGroup,
        items: updateSubMenu(menuGroup.items),
      })),
    );

    if (sidebarMinimized && menu.children) {
      toggleSidebar();
    } else if (!menu.children && menu.route) {
      navigate(menu.route);
    }
  };

  const renderMenuItem = (item: ISubMenuItem) => (
    <li key={item.label}>
      <div
        onClick={() => toggleMenu(item)}
        className="group relative text-muted-foreground cursor-pointer hover:bg-card flex h-9 items-center justify-start rounded"
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

        {item.children && !sidebarMinimized ? (
          <ExpandableMenuItem item={item} />
        ) : (
          <Link
            to={item.route!}
            className={`ml-10 truncate text-xs font-semibold tracking-wide ${item.active ? 'text-primary' : ''}`}>
            {item.label}
          </Link>
        )}

        {sidebarMinimized && (
          <div className="absolute w-full">
            <span className="z-100 absolute left-14 -top-[34px] w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover:scale-100">
              {item.label}
            </span>
          </div>
        )}
      </div>

      {item.children && <SidebarSubMenu submenu={item} toggleMenu={toggleMenu} />}
    </li>
  );

  const ExpandableMenuItem = ({ item }: { item: ISubMenuItem }) => (
    <div className="flex h-9 items-center justify-start rounded hover:bg-card">
      <span className="ml-10 truncate text-xs font-semibold tracking-wide text-muted-foreground group-hover:text-foreground">
        {item.label}
      </span>
      <button
        className={`${item.expanded ? 'rotate-90' : ''} absolute top-1 right-0 flex items-center p-1 text-muted-foreground/50 transition-all transform duration-500`}
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
