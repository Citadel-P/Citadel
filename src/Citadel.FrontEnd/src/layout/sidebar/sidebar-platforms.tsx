import { useEffect, useId } from 'react';
import { Check, ChevronRight, ChevronsUpDown, Server } from 'lucide-react';
import { Link, useLocation } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { useLocalStorage } from '@/lib/hooks';
import { PlatformMenu } from './menu-items';
import { isSidebarRouteActive } from './sidebar-routes';
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuAction,
  SidebarMenuSub,
  SidebarMenuSubItem,
  SidebarMenuSubButton,
} from '@/components/ui/sidebar';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';

export function SidebarPlatforms() {
  const { currentPlatform, platforms } = useAppContext();
  const { pathname } = useLocation();
  const [lastPlatformId, setLastPlatformId] = useLocalStorage<string | null>('sidebar-platform-id', null);
  const [expanded, setExpanded] = useLocalStorage('sidebar-platform-expanded', true);
  const submenuId = useId();
  const platformId = pathname.match(/^\/platforms\/(?:edit\/)?([^/]+)/)?.[1];
  const available = platforms ?? (currentPlatform ? [currentPlatform] : []);
  const routePlatform = available.find((platform) => platform.id === platformId);
  const selected = platformId ? routePlatform : available.find((platform) => platform.id === lastPlatformId);
  const selectedMenu = selected ? PlatformMenu(selected) : undefined;
  const resourceKind = pathname.match(/^\/platforms\/[^/]+\/([^/]+)/)?.[1];

  useEffect(() => {
    if (routePlatform) setLastPlatformId(routePlatform.id);
  }, [routePlatform, setLastPlatformId]);

  return (
    <SidebarGroup className="mb-1">
      <SidebarGroupLabel>Platform</SidebarGroupLabel>
      <SidebarMenu>
        <SidebarMenuItem>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <SidebarMenuButton
                aria-label={selected ? `Switch platform: ${selected.name}` : 'Select platform'}
                tooltip={selected?.name ?? 'Select platform'}
                className={`border border-sidebar-border ${selected ? 'pr-9' : ''}`}>
                <Server aria-hidden="true" className="size-3.5 shrink-0 text-muted-foreground" />
                <span data-sidebar-label className="min-w-0 flex-1 truncate">
                  {selected?.name ?? 'Select platform'}
                </span>
                {!selected && (
                  <ChevronsUpDown
                    aria-hidden="true"
                    className="ml-auto size-3.5 text-muted-foreground group-data-[collapsible=icon]:hidden"
                  />
                )}
              </SidebarMenuButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent side="right" align="start" className="w-64 max-w-[calc(100vw-2rem)]">
              <DropdownMenuLabel className="text-xs text-muted-foreground">Platforms</DropdownMenuLabel>
              {available.map((platform) => {
                const menu = PlatformMenu(platform);
                const equivalent = menu.children?.find((item) => item.route?.endsWith(`/${resourceKind}`));
                return (
                  <DropdownMenuItem key={platform.id} asChild onSelect={() => setExpanded(true)}>
                    <Link
                      to={equivalent?.route ?? menu.route!}
                      aria-current={selected?.id === platform.id ? 'true' : undefined}>
                      <span className="min-w-0 flex-1 truncate">{platform.name}</span>
                      {selected?.id === platform.id && <Check aria-hidden="true" className="size-3.5" />}
                    </Link>
                  </DropdownMenuItem>
                );
              })}
              <DropdownMenuSeparator />
              <DropdownMenuItem asChild>
                <Link to="/">All platforms</Link>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {selectedMenu && (
            <SidebarMenuAction
              aria-label="Platform resources"
              aria-expanded={expanded}
              aria-controls={submenuId}
              onClick={() => setExpanded((value) => !value)}
              className="right-1 top-[calc((var(--control-height)-1.5rem)/2)]">
              <ChevronRight
                aria-hidden="true"
                className={`transition-transform duration-150 motion-reduce:transition-none ${expanded ? 'rotate-90' : ''}`}
              />
            </SidebarMenuAction>
          )}
          {selectedMenu && expanded && (
            <SidebarMenuSub id={submenuId}>
              {[{ label: 'Overview', route: selectedMenu.route }, ...(selectedMenu.children ?? [])].map((item) => {
                const active = isSidebarRouteActive(pathname, item.route);
                return (
                  <SidebarMenuSubItem key={item.route}>
                    <SidebarMenuSubButton asChild isActive={active}>
                      <Link to={item.route!} aria-current={active ? 'page' : undefined}>
                        {item.label}
                      </Link>
                    </SidebarMenuSubButton>
                  </SidebarMenuSubItem>
                );
              })}
            </SidebarMenuSub>
          )}
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarGroup>
  );
}
