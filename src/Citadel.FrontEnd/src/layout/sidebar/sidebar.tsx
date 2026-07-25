import { useLayoutContext } from '@/lib/context/layout-context';
import { Info } from 'lucide-react';
import LogoIcon from '@/assets/logo.svg';
import { SidebarMenu } from './sidebar-menu';
import { Link, useNavigate } from 'react-router';
import { useRead } from '@/lib/hooks';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import {
  Sidebar as SidebarRoot,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/components/ui/sidebar';

export const Sidebar = () => {
  const navigate = useNavigate();
  const { sidebarMinimized } = useLayoutContext();
  const { data: applicationInfo } = useRead('getApplicationInfo');
  const { entitlements } = useLicenseEntitlements();
  const version = applicationInfo?.data?.version ?? '-';
  const licenseType = entitlements?.effectiveEdition ?? '-';

  return (
    <SidebarRoot collapsible="icon">
      <SidebarHeader className="h-16 justify-center border-b border-border">
        <span
          onClick={() => navigate('/')}
          onKeyDown={(e) => e.key === 'Enter' && navigate('/')}
          role="button"
          tabIndex={0}
          className="flex h-10 cursor-pointer items-center gap-3 rounded-md  focus:outline-hidden focus:ring-1 hover:bg-card">
          <span className="flex size-8 shrink-0 items-center justify-center rounded bg-primary p-2 text-background">
            <LogoIcon />
          </span>
          <b className="truncate text-sm font-bold text-foreground group-data-[collapsible=icon]:hidden">Citadel</b>
        </span>
      </SidebarHeader>

      <SidebarContent>
        <SidebarMenu />
      </SidebarContent>

      <SidebarFooter className="border-t border-border">
        {sidebarMinimized ? (
          <div className="group/version relative flex h-9 items-center justify-center rounded-md p-2 hover:bg-card">
            <Info width={18} className="text-muted-foreground/50" />
            <span className="absolute bottom-2 left-12 z-10 w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover/version:scale-100">
              v {version} - {licenseType}
            </span>
          </div>
        ) : (
          <div className="flex min-w-0 items-center justify-between gap-2 rounded-md px-2 py-1.5 text-[10px] font-semibold tracking-wide text-muted-foreground">
            <a
              target="_blank"
              rel="noreferrer"
              href="https://github.com/Citadel-P/Citadel"
              className="truncate rounded-lg bg-primary/10 px-2 text-primary hover:underline">
              v {version}
            </a>
            <Link
              to="/license"
              className="truncate rounded-lg bg-card px-2 text-muted-foreground hover:text-foreground">
              {licenseType}
            </Link>
          </div>
        )}
      </SidebarFooter>
      <SidebarRail />
    </SidebarRoot>
  );
};
