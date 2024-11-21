import LogoIcon from '@/assets/logo.svg';

export const Logo = () => {
  return (
    <div className="flex items-center justify-start sm:order-2 md:mr-10 lg:hidden">
      <button className="flex items-center text-background justify-center rounded bg-primary p-2">
        <LogoIcon />
      </button>
      <b className="hidden pl-3 text-sm font-bold text-foreground sm:block"> Citadel </b>
    </div>
  );
};
