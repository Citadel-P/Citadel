import { Link } from 'react-router';
import { useLayoutContext } from '@/lib/context/layout-context';
import { ISubMenuItem } from './menu-items';
import { ChevronRight } from 'lucide-react';
import clsx from 'clsx';

interface IProps {
  submenu: ISubMenuItem;
  toggleMenu: (menu: ISubMenuItem) => void;
}

export const SidebarSubMenu = ({ submenu, toggleMenu }: IProps) => {
  const { sidebarMinimized } = useLayoutContext();
  const expanded = !!submenu.expanded;

  if (sidebarMinimized) return null;

  return (
    <div
      aria-hidden={!expanded}
      className={clsx(
        'transition-all duration-300 ease-out overflow-hidden pl-4 pt-1',
        expanded ? 'grid grid-rows-[1fr] opacity-100' : 'grid grid-rows-[0fr] opacity-0',
      )}>
      <ul className="overflow-hidden flex flex-col border-l border-dashed border-border pl-2 gap-0.5 text-muted-foreground">
        {submenu.children?.map((sub) => (
          <li key={sub.label} className='hover:underline'>
            <SubRow sub={sub} toggleMenu={toggleMenu} />
            {sub.children && sub.expanded && <SidebarSubMenu submenu={sub} toggleMenu={toggleMenu} />}
          </li>
        ))}
      </ul>
    </div>
  );
};

function SubRow({ sub, toggleMenu }: { sub: ISubMenuItem; toggleMenu: (menu: ISubMenuItem) => void }) {
  const base = 'flex items-center rounded text-muted-foreground hover:bg-card hover:text-foreground';

  if (sub.children) {
    return (
      <button
        type="button"
        onClick={() => toggleMenu(sub)}
        onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleMenu(sub)}
        aria-expanded={!!sub.expanded}
        className={clsx(base, 'w-full px-3 py-2')}>
        <span className="flex-1 text-left text-xs font-medium">{sub.label}</span>
        <ChevronRight
          className={clsx(
            'h-4 w-4 transition-transform duration-300 ease-out text-muted-foreground/60',
            sub.expanded && 'rotate-90',
          )}
          aria-hidden
        />
      </button>
    );
  }

  return (
    <Link
      to={sub.route ?? '/'}
      className={clsx(base, 'w-full px-3 py-2 text-xs font-medium', sub.active && 'text-primary bg-card')}
      onClick={() => toggleMenu(sub)}>
      {sub.label}
    </Link>
  );
}
