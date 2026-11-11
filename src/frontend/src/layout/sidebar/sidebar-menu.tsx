import { ISubMenuItem, MenuItems } from './menu-items';
import { Link, useLocation } from 'react-router';
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
import { SidebarPlatforms } from './sidebar-platforms';
import { isSidebarRouteActive } from './sidebar-routes';

export const SidebarMenu = () => {
  const { pathname } = useLocation();
  const { unresolvedAlertCount } = useAppContext();
  const { state, isMobile } = useSidebar();
  const sidebarMinimized = state === 'collapsed' && !isMobile;
  return (
    <>
      <SidebarPlatforms />
      {MenuItems.map((menu) => (
        <SidebarGroup key={menu.group} className="py-1">
          <SidebarGroupLabel className="h-6 text-[11px] font-medium text-muted-foreground">
            {menu.group}
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenuList>
              {menu.items.map((item) => (
                <SidebarMenuItem key={item.route ?? item.label}>
                  <SidebarRow
                    item={{ ...item, active: isSidebarRouteActive(pathname, item.route) }}
                    minimized={!!sidebarMinimized}
                    badgeCount={item.route === '/alerts' ? unresolvedAlertCount : undefined}
                  />
                </SidebarMenuItem>
              ))}
            </SidebarMenuList>
          </SidebarGroupContent>
        </SidebarGroup>
      ))}
    </>
  );
};

function SidebarRow({ item, minimized, badgeCount }: { item: ISubMenuItem; minimized: boolean; badgeCount?: number }) {
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
