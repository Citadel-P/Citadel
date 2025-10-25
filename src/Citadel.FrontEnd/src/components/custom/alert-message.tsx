import { InfoIcon, Check, TriangleAlert, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { JSX, ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface AlertMessageProps {
  title?: string;
  children?: string | ReactNode;
  type: 'success' | 'info' | 'warning' | 'error';
}

export const AlertMessage = ({ type, title, children }: AlertMessageProps): JSX.Element => {
  const alertConfig = {
    success: {
      alertTitle: title ?? 'Success!',
      icon: <Check className="h-4 w-4" />,
      className: 'bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-100',
    },
    info: {
      alertTitle: title ?? 'Info',
      icon: <InfoIcon className="h-4 w-4" />,
      className: 'bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-100',
    },
    warning: {
      alertTitle: title ?? 'Heads up!',
      icon: <AlertCircle className="h-4 w-4" />,
      className: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-100',
    },
    error: {
      alertTitle: title ?? 'Error!',
      icon: <TriangleAlert className="h-4 w-4" />,
      className: 'bg-red-50 text-red-600 dark:bg-red-950 dark:text-red-100',
    },
  };

  const { alertTitle, icon, className } = alertConfig[type];

  return (
    <BaseMessage icon={icon} title={alertTitle} className={className}>
      {children}
    </BaseMessage>
  );
};

interface BaseMessageProps {
  icon: ReactNode;
  title?: string;
  className: string;
  children?: string | ReactNode;
}

const BaseMessage = ({ icon, title, className, children }: BaseMessageProps): JSX.Element => {
  return (
    <Alert className={cn('border-0 mt-2 mb-4', className)}>
      {icon}
      {title ? (
        <>
          <AlertTitle>{title}</AlertTitle>
          <AlertDescription>{children}</AlertDescription>
        </>
      ) : (
        <AlertTitle>{children}</AlertTitle>
      )}
    </Alert>
  );
};
