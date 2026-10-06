import { useAppearance } from '@/lib/appearance/appearance-context';
import { cn } from '@/lib/utils';
import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-react';
import { Toaster as Sonner, type ToasterProps } from 'sonner';
import './sonner.css';

const Toaster = ({ className, toastOptions, icons, ...props }: ToasterProps) => {
  const { effectiveMode: theme } = useAppearance();

  return (
    <Sonner
      theme={theme}
      closeButton
      richColors={false}
      className={cn('citadel-toaster', className)}
      icons={{
        success: <CircleCheck className="size-4" aria-hidden="true" />,
        error: <CircleAlert className="size-4" aria-hidden="true" />,
        info: <Info className="size-4" aria-hidden="true" />,
        warning: <TriangleAlert className="size-4" aria-hidden="true" />,
        close: <X className="size-3.5" aria-hidden="true" />,
        ...icons,
      }}
      toastOptions={{
        closeButtonAriaLabel: 'Dismiss notification',
        ...toastOptions,
      }}
      {...props}
    />
  );
};

export { Toaster };
