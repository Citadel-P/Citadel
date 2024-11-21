import { Menu } from 'lucide-react';
import { useLayoutContext } from '../LayoutProvider';
import { NavbarMobile } from './navbar-mobile/NavbarMobile';
import { Logo } from '../Logo';
import { PorfileMenu } from './profile-menu/ProfileMenu';

export const Navbar = () => {
  const { toggleMobileMenu } = useLayoutContext();

  return (
    <div className="relative bg-background">
      <div className="mx-auto px-5">
        <div className="flex items-center justify-between py-3.5 md:justify-start">
          <div className="sm:order-1 md:hidden">
            <button
              onClick={toggleMobileMenu}
              type="button"
              className="inline-flex items-center justify-center rounded-md bg-muted p-2 text-muted-foreground focus:outline-none focus:ring-2 focus:ring-inset focus:ring-primary hover:bg-muted-foreground hover:text-muted"
              aria-expanded="false">
              <span className="sr-only">Open menu</span>
              <Menu size={20} />
            </button>
          </div>
          <Logo />
          <PorfileMenu />
        </div>
      </div>
      <NavbarMobile />
    </div>
  );
};
