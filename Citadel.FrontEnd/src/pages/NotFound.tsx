import NotFound404 from '@/assets/404.svg';
import { useHideBreadcrumb } from '@/layout/breadcrumb/useHideBreadcrumb';

export default function NotFound() {
  useHideBreadcrumb();
  return (
    <div className="flex justify-center px-4 py-4 sm:py-28 sm:px-6">
      <div className="flex flex-col max-w-lg items-center gap-2 rounded-lg bg-background p-8 text-center shadow-xs">
        <h1 className="text-4xl font-bold text-foreground">Oops!</h1>
        <p className="text-base text-muted-foreground">
          We can't find that page. Please go back to the previous page or go to the homepage.
        </p>
        <NotFound404 />
      </div>
    </div>
  );
}
