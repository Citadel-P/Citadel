import { ResourceType } from '@/api/types';
import { useLayoutContext } from '@/layout/LayoutContext';
import { cn } from '@/lib/utils';
import { LucideIcon } from 'lucide-react';

export interface ActionButtonConfig {
  id: string;
  icon: LucideIcon;
  label: string;
  onClick: () => void;
  disabled: boolean;
  ariaLabel: string;
  variant?: 'default' | 'danger';
  className?: string;
}

export const ActionBar = <T,>({ selectedRows, allItems, resource, actionButtons }: ActionBarProps<T>) => {
  const { sidebarMinimized } = useLayoutContext();

  if (!selectedRows?.length) return null;

  return (
    <div
      className={`fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background flex flex-wrap justify-center items-center gap-x-4 gap-y-2 sm:justify-between ${
        sidebarMinimized ? 'action-bar-left-collapsed' : 'action-bar-left'
      }`}
      style={{
        width: sidebarMinimized ? 'calc(100% - var(--sidebar-minimized-width))' : 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {allItems?.length} {resource.toLowerCase()}(s) selected.
      </div>
      {actionButtons}
    </div>
  );
};

export const ActionButtons = ({ buttons }: ActionButtonsProps) => {
  return (
    <div className="mt-1">
      {buttons.map((button, index) => {
        const isFirst = index === 0;
        const isLast = index === buttons.length - 1;
        const isOnly = buttons.length === 1;

        let roundedClass = '';
        if (isOnly) {
          roundedClass = 'rounded-lg';
        } else if (isFirst) {
          roundedClass = 'rounded-l-lg';
        } else if (isLast) {
          roundedClass = 'rounded-r-lg';
        }

        const baseClasses =
          'inline-flex items-center border border-border px-2 py-2 font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60';

        const variantClasses =
          button.variant === 'danger'
            ? 'text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background'
            : 'text-foreground bg-background enabled:hover:bg-accent enabled:hover:text-accent-foreground';

        return (
          <ActionBarButton
            key={button.id}
            onClick={button.onClick}
            disabled={button.disabled}
            icon={button.icon}
            label={button.label}
            className={`${baseClasses} ${variantClasses} ${roundedClass} ${button.className || ''}`}
            ariaLabel={button.ariaLabel}
          />
        );
      })}
    </div>
  );
};

const ActionBarButton = ({
  onClick,
  disabled,
  icon: Icon,
  label,
  className = '',
  ariaLabel,
}: ActionBarButtonProps): React.ReactNode => (
  <button
    type="button"
    onClick={onClick}
    disabled={disabled}
    aria-label={ariaLabel}
    className={cn(
      'inline-flex items-center border border-border px-2 py-2 text-xs font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60',
      className,
    )}>
    <Icon className="mr-1 h-3 w-3" />
    {label}
  </button>
);

interface ActionBarProps<T> {
  selectedRows: T[] | undefined;
  allItems: T[] | undefined;
  resource: ResourceType;
  actionButtons: React.ReactNode;
}

interface ActionButtonsProps {
  buttons: ActionButtonConfig[];
}

interface ActionBarButtonProps {
  onClick: () => void;
  disabled: boolean;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  className?: string;
  ariaLabel?: string;
}
