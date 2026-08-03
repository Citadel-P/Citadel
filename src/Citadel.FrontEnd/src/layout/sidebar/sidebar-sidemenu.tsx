import { Link } from 'react-router';
import { useLayoutContext } from '@/lib/context/layout-context';
import { ISubMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';
import clsx from 'clsx';
import { SidebarMenuSub, SidebarMenuSubButton, SidebarMenuSubItem } from '@/components/ui/sidebar';

interface IProps {
  submenu: ISubMenuItem;
  toggleMenu: (menu: ISubMenuItem) => void;
}

export const SidebarSubMenu = ({ submenu, toggleMenu }: IProps) => {
  const { sidebarMinimized } = useLayoutContext();
  const expanded = !!submenu.expanded;

  if (sidebarMinimized || !expanded) return null;

  return (
    <SidebarMenuSub className="animate-in fade-in-0 slide-in-from-top-1 overflow-hidden duration-150">
      {submenu.children?.map((sub) => (
        <SidebarMenuSubItem key={sub.label}>
          <SubRow sub={sub} toggleMenu={toggleMenu} />
          {sub.children && sub.expanded && <SidebarSubMenu submenu={sub} toggleMenu={toggleMenu} />}
        </SidebarMenuSubItem>
      ))}
    </SidebarMenuSub>
  );
};

function SubRow({ sub, toggleMenu }: { sub: ISubMenuItem; toggleMenu: (menu: ISubMenuItem) => void }) {
  if (sub.disabled) {
    return (
      <SidebarMenuSubButton
        aria-disabled="true"
        tabIndex={-1}
        title={sub.disabledReason}
        className="cursor-not-allowed text-sidebar-foreground/40 hover:bg-transparent hover:text-sidebar-foreground/40">
        {sub.icon && <span className="flex items-center">{sub.icon}</span>}
        <span className="truncate">{sub.label}</span>
      </SidebarMenuSubButton>
    );
  }

  if (sub.children) {
    return (
      <SidebarMenuSubButton
        type="button"
        onClick={() => toggleMenu(sub)}
        onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleMenu(sub)}
        aria-expanded={!!sub.expanded}>
        {sub.icon && <span className="flex items-center">{sub.icon}</span>}
        <span className="flex-1 text-left">{sub.label}</span>
        <ChevronRight
          className={clsx(
            'size-3.5 text-sidebar-foreground/40 transition-transform duration-200 ease-out',
            sub.expanded && 'rotate-90',
          )}
          aria-hidden
        />
      </SidebarMenuSubButton>
    );
  }

  return (
    <SidebarMenuSubButton asChild isActive={sub.active}>
      <Link to={sub.route ?? '/'} onClick={() => toggleMenu(sub)}>
        {sub.icon && <span className="flex items-center">{sub.icon}</span>}
        <span className="truncate">{sub.label}</span>
      </Link>
    </SidebarMenuSubButton>
  );
}
