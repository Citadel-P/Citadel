import { useCallback, useEffect, useState, useRef, Fragment, useMemo } from 'react';
import { useLayoutContext } from '@/lib/context/layout-context';
import { ISubMenuItem, MenuItems, DockerPlatformMenu, IMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation, useNavigate } from 'react-router';
import { SidebarSubMenu } from './sidebar-sidemenu';
import { useAppContext } from '@/lib/context/app-context';
import clsx from 'clsx';

export const SidebarMenu = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { currentPlatform, unresolvedAlertCount } = useAppContext();
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
        <div className="pt-4" key={menu.group || i}>
          {!sidebarMinimized && (
            <div className="mx-1 mb-2 flex items-center justify-between">
              <small className="text-xs font-medium text-muted-foreground/50">{menu.group}</small>
            </div>
          )}

          <ul className="flex flex-col space-y-1">
            {menu.items.map((item) => (
              <li key={item.label}>
                <SidebarRow
                  item={item}
                  minimized={!!sidebarMinimized}
                  onClick={() => toggleMenu(item)}
                  badgeCount={item.route === '/alerts' ? unresolvedAlertCount : undefined}
                />
                {item.children && <SidebarSubMenu submenu={item} toggleMenu={toggleMenu} />}
              </li>
            ))}
          </ul>

          {menu.separator && (
            <div className="pt-3">
              <hr className="border-dashed border-border" />
            </div>
          )}
        </div>
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

  return (
    <div
      className={clsx(
        'group relative flex items-center h-9 gap-3 rounded cursor-pointer hover:bg-card text-muted-foreground px-2',
        item.active && 'text-primary bg-card',
      )}
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && onClick()}>
      {/* Icon */}
      <div className={clsx('relative', item.active ? 'text-primary' : 'text-muted-foreground')}>
        {item.icon}
        {hasBadge && (
          <span className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-destructive ring-1 ring-background" />
        )}
      </div>

      {/* Label section (hidden when minimized) */}
      {!minimized && (
        <div className={'flex items-center hover:underline w-full'}>
          {item.children ? (
            <ExpandableHead label={item.label} expanded={!!item.expanded} />
          ) : (
            <div className="flex w-full items-center justify-between gap-2">
              <Link
                to={item.route ?? '/'}
                className={clsx('truncate text-xs font-medium', item.active && 'text-primary ')}>
                {item.label}
              </Link>
              {hasBadge && (
                <span className="text-[10px] font-medium tabular-nums leading-none text-muted-foreground/60">
                  {badgeLabel}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {/* Tooltip when collapsed */}
      {minimized && (
        <div className="absolute inset-0 flex items-center justify-center group">
          <div
            className="
              absolute left-14
              top-1/2 -translate-y-1/2
              scale-0 group-hover:scale-100
              transition-transform duration-150 origin-left
              bg-foreground text-background
              text-xs font-medium
              rounded-md shadow-md
              p-2 whitespace-nowrap
              pointer-events-auto
              z-50
            ">
            <div className="flex items-center gap-2">
              <span>{item.label}</span>
              {hasBadge && (
                <span className="text-[10px] font-medium tabular-nums leading-none text-muted-foreground/60">
                  {badgeLabel}
                </span>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function ExpandableHead({ label, expanded }: { label: string; expanded: boolean }) {
  return (
    <div className="flex h-9 items-center justify-between rounded hover:bg-card w-full pr-1">
      <span className="truncate text-xs font-medium text-muted-foreground group-hover:text-foreground">{label}</span>
      <ChevronRight
        className={clsx(
          'h-4 w-4 transition-transform duration-300 ease-out text-muted-foreground/60',
          expanded && 'rotate-90',
        )}
        aria-hidden
      />
    </div>
  );
}
