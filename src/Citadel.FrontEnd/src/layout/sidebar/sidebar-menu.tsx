import { useCallback, useEffect, useState, useRef, Fragment, useMemo } from 'react';
import { useLayoutContext } from '@/lib/context/layout-context';
import { ISubMenuItem, MenuItems, DockerPlatformMenu, IMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation, useNavigate } from 'react-router';
import { SidebarSubMenu } from './sidebar-sidemenu';
import { useAppContext } from '@/lib/context/app-context';
import clsx from 'clsx';
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu as SidebarMenuList,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from '@/components/ui/sidebar';

export const SidebarMenu = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { currentPlatform, platforms, unresolvedAlertCount } = useAppContext();
  const { toggleSidebar, sidebarMinimized } = useLayoutContext();

  const [menuItems, setMenuItems] = useState<IMenuItem[]>(MenuItems);
  const addedPlatformIdsRef = useRef<Set<string>>(new Set());

  const isRouteActive = useCallback(
    (path: string) => location.pathname === path || (location.pathname === '/' && path === ''),
    [location.pathname],
  );

  const addPlatformToMenu = useCallback(
    (platform: { id: string; name: string }) => {
      if (!platform?.id || addedPlatformIdsRef.current.has(platform.id)) return;

      const platformRoute = `/platforms/edit/${platform.id}`;
      const platformMenu = DockerPlatformMenu(platform);

      platformMenu.children?.forEach((item) => {
        item.active = isRouteActive(item.route ?? '');
      });
      platformMenu.expanded = platformMenu.children?.some((menu) => menu.active) || false;
      addedPlatformIdsRef.current.add(platform.id);

      setMenuItems((prev) => {
        const baseMenuIndex = prev.findIndex((menu) => menu.group === 'Infrastructure');
        if (baseMenuIndex === -1) return prev;

        const baseMenu = prev[baseMenuIndex];
        if (baseMenu.items.some((i) => i.route === platformRoute)) return prev;

        const updatedBase = { ...baseMenu, items: [...baseMenu.items, platformMenu] };
        return [...prev.slice(0, baseMenuIndex), updatedBase, ...prev.slice(baseMenuIndex + 1)];
      });
    },
    [isRouteActive],
  );

  // Clean up removed platforms
  useEffect(() => {
    const baseMenu = menuItems.find((menu) => menu.group === 'Infrastructure');
    if (!baseMenu) return;

    const ids = new Set<string>();
    baseMenu.items.forEach((item) => {
      const match = item.route?.match(/\/platforms\/(?:edit\/)?([^/]+)/);
      if (match?.[1]) ids.add(match[1]);
    });
    addedPlatformIdsRef.current = ids;
  }, [menuItems]);

  const displayedMenuItems = useMemo(() => {
    const withRouteState = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((item) => {
        const children = item.children ? withRouteState(item.children) : undefined;
        const hasActiveChild = children ? children.some((c) => c.active) : false;
        const isActive = isRouteActive(item.route ?? '');

        return {
          ...item,
          active: isActive,
          expanded: item.children ? (item.expanded ?? hasActiveChild) : isActive,
          children,
        };
      });

    return menuItems.map((menu) => ({
      ...menu,
      items: withRouteState(menu.items),
    }));
  }, [menuItems, isRouteActive]);

  useEffect(() => {
    if (currentPlatform?.id) {
      addPlatformToMenu({ id: currentPlatform.id, name: currentPlatform.name ?? '' });
    }
  }, [currentPlatform, addPlatformToMenu]);

  useEffect(() => {
    if (!platforms) return;

    const livePlatformIds = new Set(platforms.map((platform) => platform.id).filter(Boolean));

    setMenuItems((prev) => {
      let changed = false;

      const next = prev.map((menu) => {
        if (menu.group !== 'Infrastructure') return menu;

        const items = menu.items.filter((item) => {
          if (!item.isPlatform) return true;

          const platformId = getMenuPlatformId(item);
          const keep = !!platformId && livePlatformIds.has(platformId);

          if (!keep) {
            changed = true;
            if (platformId) addedPlatformIdsRef.current.delete(platformId);
          }

          return keep;
        });

        return changed ? { ...menu, items } : menu;
      });

      return changed ? next : prev;
    });
  }, [platforms]);

  const toggleMenu = (menu: ISubMenuItem) => {
    const targetLabel = menu.label;
    const targetRoute = menu.route;

    const update = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.map((i) => ({
        ...i,
        expanded: i.label === targetLabel && i.route === targetRoute ? !i.expanded : i.expanded,
        children: i.children ? update(i.children) : undefined,
      }));

    setMenuItems((prev) =>
      prev.map((group) => ({
        ...group,
        items: update(group.items),
      })),
    );

    if (sidebarMinimized && menu.children) toggleSidebar();
    else if (!menu.children && menu.route) navigate(menu.route);
  };

  return (
    <Fragment>
      {displayedMenuItems.map((menu, i) => (
        <SidebarGroup key={menu.group || i}>
          <SidebarGroupLabel>{menu.group}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenuList className="pl-1 group-data-[collapsible=icon]:pl-0">
              {menu.items.map((item) => (
                <SidebarMenuItem key={item.label}>
                  <SidebarRow
                    item={item}
                    minimized={!!sidebarMinimized}
                    onClick={() => toggleMenu(item)}
                    badgeCount={item.route === '/alerts' ? unresolvedAlertCount : undefined}
                  />
                  {item.children && <SidebarSubMenu submenu={item} toggleMenu={toggleMenu} />}
                </SidebarMenuItem>
              ))}
            </SidebarMenuList>
          </SidebarGroupContent>

          {menu.separator && <SidebarSeparator className="mt-3 border-dashed" />}
        </SidebarGroup>
      ))}
    </Fragment>
  );
};

function SidebarRow({
  item,
  minimized,
  onClick,
  badgeCount,
}: {
  item: ISubMenuItem;
  minimized: boolean;
  onClick: () => void;
  badgeCount?: number;
}) {
  const hasBadge = !!badgeCount;
  const badgeLabel = badgeCount && badgeCount > 99 ? '99+' : badgeCount?.toString();
  const icon = (
    <span
      data-sidebar-icon
      className={clsx(
        'relative flex size-3.5 shrink-0 items-center justify-center [&>svg]:size-3.5',
        item.active ? 'text-primary' : 'text-muted-foreground',
      )}>
      {item.icon}
      {hasBadge && (
        <span className="absolute -right-0.5 -top-0.5 size-1.5 rounded-full bg-destructive ring-1 ring-background" />
      )}
    </span>
  );

  if (item.children) {
    return (
      <SidebarMenuButton
        tooltip={item.label}
        isActive={item.active}
        onClick={onClick}
        className={clsx('text-muted-foreground', hasBadge && 'pr-10')}
        aria-expanded={!!item.expanded}>
        {icon}
        <span data-sidebar-label className="truncate">
          {item.label}
        </span>
        <ChevronRight
          className={clsx(
            'ml-auto size-4 text-muted-foreground/60 transition-transform duration-300 ease-out group-data-[collapsible=icon]:hidden',
            item.expanded && 'rotate-90',
          )}
          aria-hidden
        />
        {hasBadge && !minimized && <SidebarMenuBadge>{badgeLabel}</SidebarMenuBadge>}
      </SidebarMenuButton>
    );
  }

  return (
    <>
      <SidebarMenuButton asChild tooltip={item.label} isActive={item.active} className={clsx(hasBadge && 'pr-10')}>
        <Link to={item.route ?? '/'} onClick={onClick}>
          {icon}
          <span data-sidebar-label className="truncate">
            {item.label}
          </span>
        </Link>
      </SidebarMenuButton>
      {hasBadge && !minimized && <SidebarMenuBadge>{badgeLabel}</SidebarMenuBadge>}
    </>
  );
}

const getMenuPlatformId = (item: ISubMenuItem) => item.route?.match(/^\/platforms\/edit\/([^/]+)/)?.[1];
