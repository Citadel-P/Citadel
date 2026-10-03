import { InfoIcon, Check, TriangleAlert, AlertCircle, X, Clock } from 'lucide-react';
import { JSX, ReactNode, useState } from 'react';
import { cn } from '@/lib/utils';
import { fromNow } from '@/lib/dayjs.helper';

interface AlertMessageProps {
  title?: string;
  children?: string | ReactNode;
  date?: Date | number | string;
  type: 'success' | 'info' | 'warning' | 'error';
  dismissible?: boolean;
  onDismiss?: () => void;
  className?: string;
}

export const AlertMessage = ({
  type,
  title,
  children,
  dismissible,
  onDismiss,
  className,
  date,
}: AlertMessageProps): JSX.Element | null => {
  const [dismissed, setDismissed] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  if (dismissed) return null;

  const handleDismiss = () => {
    setDismissed(true);
    onDismiss?.();
  };

  const alertConfig = {
    success: {
      alertTitle: title,
      Icon: Check,
      containerClass: 'border-emerald-200 bg-emerald-50/50 dark:border-emerald-900/50 dark:bg-emerald-950/30',
      iconClass: 'text-emerald-500',
      closeClass:
        'hover:bg-emerald-100/50 hover:text-emerald-600 dark:hover:bg-emerald-900/50 dark:hover:text-emerald-400',
    },
    info: {
      alertTitle: title,
      Icon: InfoIcon,
      containerClass: 'border-blue-200 bg-blue-50/50 dark:border-blue-900/50 dark:bg-blue-950/30',
      iconClass: 'text-blue-500',
      closeClass: 'hover:bg-blue-100/50 hover:text-blue-600 dark:hover:bg-blue-900/50 dark:hover:text-blue-400',
    },
    warning: {
      alertTitle: title,
      Icon: AlertCircle,
      containerClass: 'border-amber-200 bg-amber-50/50 dark:border-amber-900/50 dark:bg-amber-950/30',
      iconClass: 'text-amber-500',
      closeClass: 'hover:bg-amber-100/50 hover:text-amber-600 dark:hover:bg-amber-900/50 dark:hover:text-amber-400',
    },
    error: {
      alertTitle: title,
      Icon: TriangleAlert,
      containerClass: 'border-red-200 bg-red-50/50 dark:border-red-900/50 dark:bg-red-950/30',
      iconClass: 'text-red-500',
      closeClass: 'hover:bg-red-100/50 hover:text-red-600 dark:hover:bg-red-900/50 dark:hover:text-red-400',
    },
  };

  const { alertTitle, Icon, containerClass, iconClass, closeClass } = alertConfig[type];

  return (
    <div
      className={cn(
        'flex justify-between gap-4 rounded-md border px-4 py-3 my-2 w-full transition-all',
        isExpanded ? 'items-start' : 'items-center',
        containerClass,
        className,
      )}>
      <div className={cn('flex gap-3', isExpanded ? 'items-start w-full' : 'items-center overflow-hidden')}>
        <Icon className={cn('h-4 w-4 shrink-0', iconClass, isExpanded && 'mt-0.5')} />
        <div
          className={cn(
            'flex text-sm text-foreground/90 transition-all',
            isExpanded ? 'flex-col items-start gap-1 w-full' : 'items-center gap-2 truncate',
          )}>
          {alertTitle && <span className="font-medium text-foreground/90 shrink-0">{alertTitle}</span>}
          {alertTitle && children && !isExpanded && (
            <span className="text-muted-foreground/40 hidden sm:inline shrink-0">•</span>
          )}
          {children && (
            <div
              className={cn(
                'text-muted-foreground transition-colors',
                isExpanded
                  ? 'whitespace-normal w-full'
                  : 'truncate cursor-pointer hover:text-muted-foreground/80 w-full',
              )}
              onClick={() => !isExpanded && setIsExpanded(true)}
              title={isExpanded ? undefined : 'Click to expand'}>
              {children}
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 ml-2">
        {date && (
          <div className="flex flex-wrap gap-2 items-center text-muted-foreground">
            <Clock width={13} height={13} />
            <span className="text-xs">{fromNow(date)}</span>
          </div>
        )}

        {dismissible && (
          <button
            onClick={handleDismiss}
            className={cn('shrink-0 rounded-md p-1 text-muted-foreground transition-colors', closeClass)}
            aria-label="Dismiss">
            <X className="h-4 w-4" />
          </button>
        )}
      </div>
    </div>
  );
};
