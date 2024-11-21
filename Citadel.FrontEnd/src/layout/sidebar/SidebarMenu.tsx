import { useEffect, useState } from 'react';
import { useLayoutContext } from '../LayoutProvider';
import { ISubMenuItem, MenuItems } from './menu-items';
import { ChevronRight } from 'lucide-react';
import { Link, useLocation } from 'react-router-dom';
import { SidebarSubMenu } from './SidebarSidemenu';
import { useNavigate } from 'react-router-dom';

export const SidebarMenu = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { sidebarMinimized, toggleSidebar } = useLayoutContext();
  const [menuItems, setMenuItems] = useState(MenuItems);

  useEffect(() => {
    // Set dynamic menu
    menuItems.forEach((menu) => {
      let activeGroup = false;
      menu.items.forEach((subMenu) => {
        const active = isRouteActive(subMenu.route ?? '');
        subMenu.expanded = active;
        subMenu.active = active;
        if (active) activeGroup = true;
        if (subMenu.children) {
          expand(subMenu.children);
        }
      });
      menu.active = activeGroup;
    });
    setMenuItems([...menuItems]);
  }, [location]);

  const expand = (items: Array<any>) => {
    items.forEach((item) => {
      item.expanded = isRouteActive(item.route);
      if (item.children) expand(item.children);
    });
  };

  const isRouteActive = (path: string) => location.pathname == path || (location.pathname == '/' && path == '');

  const toggleMenu = (menu: ISubMenuItem): void => {
    menu.expanded = !menu.expanded;
    if (sidebarMinimized && menu.children) toggleSidebar();
    else if (!menu.children) navigate(menu.route!);
    setMenuItems([...menuItems]);
  };

  return (
    <>
      {menuItems.map((menu, i) => (
        <div className="pt-4" key={i}>
          <div className="mx-1 mb-2 flex items-center justify-between">
            <small className={`${sidebarMinimized ? 'hidden' : ''} text-xs font-semibold text-muted-foreground/50`}>
              {menu.group}
            </small>
          </div>

          <ul className="flex flex-col space-y-1">
            {menu.items.map((item, j) => (
              <li key={j}>
                <div onClick={() => toggleMenu(item)} className="group relative text-muted-foreground">
                  <div
                    className={`${item.active && sidebarMinimized ? 'text-primary' : 'text-muted-foreground/50'} pointer-events-none absolute m-2`}>
                    {item.icon}
                  </div>

                  {item.children ? (
                    <div className="flex h-9 cursor-pointer items-center justify-start rounded hover:bg-card">
                      <a className="ml-10 truncate text-xs font-semibold tracking-wide text-muted-foreground focus:outline-none group-hover:text-foreground">
                        {item.label}
                      </a>
                    </div>
                  ) : (
                    <div className="flex h-9 cursor-pointer items-center justify-start rounded text-muted-foreground hover:bg-card hover:text-foreground">
                      <Link
                        to={item.route!}
                        className={`${item.active ? 'text-primary' : ''} ml-10 truncate text-xs font-semibold tracking-wide focus:outline-none`}>
                        {item.label}
                      </Link>
                    </div>
                  )}

                  {/* Arrow Icon */}
                  {item.children && (
                    <button
                      className={`${sidebarMinimized ? 'hidden' : ''} ${item.expanded ? 'rotate-90' : ''} pointer-events-none absolute top-1 right-0 flex items-center p-1 text-muted-foreground/50 transition-all duration-500`}>
                      <ChevronRight width={18} height={18} />
                    </button>
                  )}

                  {/* Tooltip */}
                  {sidebarMinimized && (
                    <div className="fixed w-full">
                      <span className="z-1 absolute left-14 -top-[34px] w-auto min-w-max origin-left scale-0 rounded-md bg-foreground p-2 text-xs font-bold text-background shadow-md transition-all duration-200 group-hover:scale-100">
                        {item.label}
                      </span>
                    </div>
                  )}
                </div>

                {/* Submenu items */}
                <SidebarSubMenu submenu={item} toggleMenu={toggleMenu} />
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
    </>
  );
};
