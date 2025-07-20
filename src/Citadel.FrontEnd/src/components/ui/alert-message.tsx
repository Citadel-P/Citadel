import { InfoIcon, Check, TriangleAlert, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ReactNode } from 'react';

interface IProps {
  type: 'success' | 'info' | 'warning' | 'error';
  hasTitle?: boolean;
  children?: string | ReactNode;
}

export const AlertMessage = ({ type, hasTitle, children }: IProps) => {
  const alertConfig = {
    success: {
      title: 'Success!',
      icon: <Check className="h-4 w-4" />,
      className: 'bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-100',
    },
    info: {
      title: 'Info!',
      icon: <InfoIcon className="h-4 w-4" />,
      className: 'bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-100',
    },
    warning: {
      title: 'Heads up!',
      icon: <AlertCircle className="h-4 w-4" />,
      className: 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-100',
    },
    error: {
      title: 'Error!',
      icon: <TriangleAlert className="h-4 w-4" />,
      className: 'bg-red-50 text-red-600 dark:bg-red-950 dark:text-red-100',
    },
  };

  const { title, icon, className } = alertConfig[type];

  return (
    <BaseMessage icon={icon} title={title} className={className} hasTitle={hasTitle}>
      {children}
    </BaseMessage>
  );
};

interface IBaseMessageProps {
  icon: ReactNode;
  title: string;
  className: string;
  hasTitle?: boolean;
  children?: string | ReactNode;
}

const BaseMessage = ({ icon, title, className, hasTitle = false, children }: IBaseMessageProps) => {
  return (
    <Alert className={`border-0 mt-2 mb-4 ${className}`}>
      {icon}
      {hasTitle ? (
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
