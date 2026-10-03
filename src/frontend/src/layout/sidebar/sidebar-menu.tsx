import { useState, useMemo } from 'react';
import { ISubMenuItem, MenuItems } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation } from 'react-router';
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
  useSidebar,
} from '@/components/ui/sidebar';
import { useRead } from '@/lib/hooks';
import { SidebarPlatforms } from './sidebar-platforms';
import { isSidebarRouteActive } from './sidebar-routes';

export const SidebarMenu = () => {
  const { pathname } = useLocation();
  const { unresolvedAlertCount } = useAppContext();
  const { toggleSidebar, state, isMobile } = useSidebar();
  const sidebarMinimized = state === 'collapsed' && !isMobile;
  const { data: profileResponse } = useRead('getCurrentProfile');
  const authorization = profileResponse?.data?.authorization;
  const [expansion, setExpansion] = useState<{ pathname: string; items: Record<string, boolean> }>({
    pathname,
    items: {},
  });

  const displayedMenuItems = useMemo(() => {
    const canDisplay = (item: ISubMenuItem) => {
      switch (item.access) {
        case 'administrator':
          return authorization?.isAdministrator === true;
        case 'alertRules':
          return authorization?.alertRules?.canRead === true;
        case 'bindings':
          return authorization?.bindings?.canRead === true;
        case 'tags':
          return authorization?.tags?.canRead === true;
        default:
          return true;
      }
    };
    const withRouteState = (items: ISubMenuItem[]): ISubMenuItem[] =>
      items.flatMap((item) => {
        if (!canDisplay(item)) return [];
        const children = item.children ? withRouteState(item.children) : undefined;
        if (item.children && !children?.length) return [];
        const active = isSidebarRouteActive(pathname, item.route) || !!children?.some((child) => child.active);
        const explicitExpansion =
          expansion.pathname === pathname ? expansion.items[item.route ?? item.label] : undefined;
        return [{ ...item, active, children, expanded: explicitExpansion ?? active }];
      });
    return MenuItems.map((menu) => ({ ...menu, items: withRouteState(menu.items) })).filter(
      (menu) => menu.items.length,
    );
  }, [authorization, pathname, expansion]);

  const toggleMenu = (menu: ISubMenuItem) => {
    if (menu.disabled || !menu.children) return;
    setExpansion((previous) => ({
      pathname,
      items: { ...(previous.pathname === pathname ? previous.items : {}), [menu.route ?? menu.label]: !menu.expanded },
    }));
    if (sidebarMinimized) toggleSidebar();
  };

  return (
    <>
      <SidebarPlatforms />
      {displayedMenuItems.map((menu) => (
        <SidebarGroup key={menu.group}>
          <SidebarGroupLabel>{menu.group}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenuList>
              {menu.items.map((item) => (
                <SidebarMenuItem key={item.route ?? item.label}>
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
        </SidebarGroup>
      ))}
    </>
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
        item.active ? 'text-sidebar-foreground' : 'text-muted-foreground',
      )}>
      {item.icon}
      {hasBadge && minimized && (
        <span className="absolute -right-0.5 -top-0.5 size-1.5 rounded-full bg-destructive ring-1 ring-background" />
      )}
    </span>
  );

  if (item.children) {
    return (
      <SidebarMenuButton
        aria-label={item.label}
        tooltip={item.label}
        isActive={item.active}
        onClick={onClick}
        className={clsx(hasBadge && 'pr-10')}
        aria-expanded={!!item.expanded}>
        {icon}
        <span data-sidebar-label className="truncate">
          {item.label}
        </span>
        <ChevronRight
          className={clsx(
            'ml-auto size-3.5 text-sidebar-foreground/40 transition-transform duration-200 ease-out group-data-[collapsible=icon]:hidden',
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
        <Link to={item.route ?? '/'} aria-label={item.label} aria-current={item.active ? 'page' : undefined}>
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
