import { Check, ChevronRight } from 'lucide-react';
import { Link, useLocation } from 'react-router';
import { useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { SidebarGroup, SidebarMenu, SidebarMenuButton, SidebarMenuItem, useSidebar } from '@/components/ui/sidebar';
import { UtilityItems, type ISubMenuItem } from './menu-items';
import { isSidebarRouteActive } from './sidebar-routes';

export function SidebarUtilities() {
  const { pathname } = useLocation();
  const { isMobile, setOpenMobile } = useSidebar();
  const { data } = useRead('getCurrentProfile');
  const authorization = data?.data?.authorization;
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
  const menus = UtilityItems.filter(canDisplay)
    .map((item) => ({ ...item, children: item.children?.filter(canDisplay) ?? [] }))
    .filter((item) => item.children.length > 0);

  if (!menus.length) return null;

  return (
    <SidebarGroup className="shrink-0 border-t border-sidebar-border py-2" aria-label="Utilities">
      <SidebarMenu>
        {menus.map((menu) => {
          const active = menu.children.some((item) => isSidebarRouteActive(pathname, item.route));
          return (
            <SidebarMenuItem key={menu.label}>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <SidebarMenuButton aria-label={menu.label} tooltip={menu.label} isActive={active}>
                    <span className="flex size-3.5 shrink-0 items-center justify-center [&>svg]:size-3.5">
                      {menu.icon}
                    </span>
                    <span data-sidebar-label className="flex-1 truncate">
                      {menu.label}
                    </span>
                    <ChevronRight className="size-3.5 group-data-[collapsible=icon]:hidden" aria-hidden="true" />
                  </SidebarMenuButton>
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  side={isMobile ? 'top' : 'right'}
                  align="end"
                  sideOffset={8}
                  collisionPadding={12}
                  className="w-56 max-w-[calc(100vw-2rem)]">
                  {menu.children.map((item) => {
                    const selected = isSidebarRouteActive(pathname, item.route);
                    return (
                      <DropdownMenuItem
                        key={item.route}
                        asChild
                        className={cn(selected && 'bg-accent font-medium text-accent-foreground')}
                        onSelect={() => {
                          if (isMobile) setOpenMobile(false);
                        }}>
                        <Link to={item.route!} aria-current={selected ? 'page' : undefined}>
                          {item.icon}
                          <span className="flex-1">{item.label}</span>
                          {selected && <Check className="size-3.5" aria-hidden="true" />}
                        </Link>
                      </DropdownMenuItem>
                    );
                  })}
                </DropdownMenuContent>
              </DropdownMenu>
            </SidebarMenuItem>
          );
        })}
      </SidebarMenu>
    </SidebarGroup>
  );
}
