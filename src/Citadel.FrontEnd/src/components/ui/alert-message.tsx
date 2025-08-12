import { InfoIcon, Check, TriangleAlert, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { JSX, ReactNode } from 'react';
import { cn } from '@/lib/utils';

/**
 * Props for the AlertMessage component.
 */
interface AlertMessageProps {
  /**
   * The type of the alert, which determines the color and icon.
   */
  type: 'success' | 'info' | 'warning' | 'error';
  /**
   * If true, the alert will have a title.
   * The title is predefined based on the `type`.
   */
  hasTitle?: boolean;
  /**
   * The content of the alert message.
   * If `hasTitle` is true, this will be the description. Otherwise, it will be the title.
   */
  children?: string | ReactNode;
}

/**
 * A component to display different types of alert messages (success, info, warning, error).
 *
 * @param {AlertMessageProps} props The props for the component.
 * @returns {JSX.Element} The rendered alert message component.
 */
export const AlertMessage = ({ type, hasTitle, children }: AlertMessageProps): JSX.Element => {
  const alertConfig = {
    success: {
      title: 'Success!',
      icon: <Check className="h-4 w-4" />,
      className: 'bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-100',
    },
    info: {
      title: 'Info',
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

/**
 * Props for the BaseMessage component.
 */
interface BaseMessageProps {
  /**
   * The icon to display in the alert.
   */
  icon: ReactNode;
  /**
   * The title of the alert.
   */
  title: string;
  /**
   * Additional CSS class names to apply to the alert.
   */
  className: string;
  /**
   * If true, the alert will have a title and a description.
   */
  hasTitle?: boolean;
  /**
   * The content of the alert message.
   */
  children?: string | ReactNode;
}

/**
 * The base component for rendering an alert message.
 * It is used by the `AlertMessage` component.
 *
 * @param {BaseMessageProps} props The props for the component.
 * @returns {JSX.Element} The rendered base message component.
 */
const BaseMessage = ({ icon, title, className, hasTitle = false, children }: BaseMessageProps): JSX.Element => {
  return (
    <Alert className={cn('border-0 mt-2 mb-4', className)}>
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