import { X } from 'lucide-react';
import { Logo } from '../../Logo';
import { useContextSelector } from 'use-context-selector';
import { LayoutContext } from '@/layout/LayoutProvider';

export const NavbarMobile = () => {
  const mobileMenuVisible = useContextSelector(LayoutContext, (v) => v?.mobileMenuVisible);
  const toggleMobileMenu = useContextSelector(LayoutContext, (v) => v?.toggleMobileMenu);

  return (
    <div
      className={`${
        mobileMenuVisible
          ? 'pointer-events-auto scale-100 animate-fade-in-up opacity-100 duration-200'
          : 'pointer-events-none scale-95 opacity-0 duration-200 ease-out'
      }`}>
      <div className="absolute inset-x-0 top-0 z-10 origin-top-right transform p-2 transition md:hidden">
        <div className="rounded-lg bg-background shadow-lg">
          <div className="pt-5 pb-6">
            <div className="flex items-center justify-between px-5">
              <div>
                <Logo />
              </div>
              <div className="-mr-2">
                <button
                  onClick={toggleMobileMenu}
                  type="button"
                  className="inline-flex items-center justify-center rounded-md p-2 text-muted-foreground transition-transform focus:outline-hidden focus:ring-0 focus:ring-inset hover:rotate-90 hover:bg-card hover:text-foreground">
                  <span className="sr-only">Close menu</span>
                  <X />
                </button>
              </div>
            </div>
            <div className="scrollbar-thumb-rounded scrollbar-track-rounded max-h-[500px] overflow-y-auto px-5 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
              {/*Mobile Menu Component*/}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
