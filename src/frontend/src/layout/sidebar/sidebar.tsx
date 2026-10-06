import { Info } from 'lucide-react';
import LogoIcon from '@/assets/logo.svg?react';
import { SidebarMenu } from './sidebar-menu';
import { Link } from 'react-router';
import { useRead } from '@/lib/hooks';
import { useAppContext } from '@/lib/context/app-context';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import {
  Sidebar as SidebarRoot,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
  SidebarMenuButton,
  useSidebar,
} from '@/components/ui/sidebar';

export const Sidebar = () => {
  const { state, isMobile } = useSidebar();
  const sidebarMinimized = state === 'collapsed' && !isMobile;
  const { applicationInfo } = useAppContext();
  const { data: profileResponse } = useRead('getCurrentProfile');
  const { entitlements } = useLicenseEntitlements();
  const version = applicationInfo?.version ?? '-';
  const licenseType = entitlements?.effectiveEdition ?? '-';
  const isAdministrator = profileResponse?.data?.authorization.isAdministrator === true;

  return (
    <SidebarRoot collapsible="icon">
      <SidebarHeader className="h-16 justify-center border-b border-sidebar-border">
        <Link
          to="/"
          aria-label="Citadel home"
          className="flex h-10 cursor-pointer items-center gap-3 rounded-md focus:outline-hidden focus:ring-2 focus:ring-sidebar-ring/50 hover:bg-sidebar-accent">
          <span className="flex size-8 shrink-0 items-center justify-center">
            <LogoIcon className="size-8" />
          </span>
          <b className="truncate text-sm font-bold text-foreground group-data-[collapsible=icon]:hidden">Citadel</b>
        </Link>
      </SidebarHeader>

      <SidebarContent className="sidebar-scroll-area">
        <SidebarMenu />
      </SidebarContent>

      <SidebarFooter className="h-[var(--layout-footer-height)] shrink-0 justify-center border-t border-sidebar-border">
        {sidebarMinimized ? (
          <SidebarMenuButton asChild tooltip={`Version ${version} · ${licenseType}`}>
            <a
              href="https://github.com/Citadel-P/Citadel"
              target="_blank"
              rel="noreferrer"
              aria-label={`Citadel version ${version}, ${licenseType} edition`}>
              <Info className="size-3.5 text-muted-foreground" aria-hidden="true" />
            </a>
          </SidebarMenuButton>
        ) : (
          <div className="flex min-w-0 items-center justify-between gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground">
            <a
              target="_blank"
              rel="noreferrer"
              href="https://github.com/Citadel-P/Citadel"
              title={`v ${version}`}
              className="min-w-0 max-w-[16ch] truncate hover:text-foreground hover:underline">
              v {version}
            </a>
            {isAdministrator ? (
              <Link to="/license" className="shrink-0 whitespace-nowrap hover:text-foreground">
                {licenseType}
              </Link>
            ) : (
              <span className="shrink-0 whitespace-nowrap hover:text-foreground">{licenseType}</span>
            )}
          </div>
        )}
      </SidebarFooter>
      <SidebarRail />
    </SidebarRoot>
  );
};
