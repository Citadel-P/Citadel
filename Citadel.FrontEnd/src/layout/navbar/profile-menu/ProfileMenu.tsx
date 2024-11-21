import { PorfileButton } from './ProfileButton';
import { PorfileDropDown } from './ProfileDropDown';
import useAnimation from '@/hooks/useAnimation';

export const PorfileMenu = () => {
  const { ref, open, setOpen } = useAnimation('dropDown');

  return (
    <div className="items-center justify-end sm:order-4 md:flex md:flex-1 lg:w-0">
      <div ref={ref} className="relative ml-3">
        <PorfileButton onClick={() => setOpen(!open)} />
        {open && <PorfileDropDown />}
      </div>
    </div>
  );
};
