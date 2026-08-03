import { ResourceType } from '@/api/types';
import { useLayoutContext } from '@/lib/context/layout-context';
import { useSelectedResources } from '@/lib/atoms';
import { cn } from '@/lib/utils';
import { ButtonActionComponent, ButtonGroupComponent } from '@/pages/types';
import { ButtonGroup } from '@/components/ui/button-group';
import { LucideIcon } from 'lucide-react';

const RESPONSIVE_BUTTON_GROUP_CLASS =
  'grid w-full grid-cols-2 gap-2 [&>*]:w-full [&>*]:max-w-none [&>*]:rounded-sm! [&>*]:border-l! sm:flex sm:w-fit sm:gap-0 sm:[&>*]:w-auto sm:[&>*]:max-w-47.5 sm:[&>*:not(:first-child)]:rounded-l-none! sm:[&>*:not(:first-child)]:border-l-0! sm:[&>*:not(:last-child)]:rounded-r-none!';

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

export const ActionBar = <T,>({
  type,
  items,
  actions,
  standaloneActions = [],
}: {
  type: ResourceType;
  items: T[];
  actions: ButtonGroupComponent<T>[];
  standaloneActions?: ButtonGroupComponent<T>[];
}) => {
  const [selectedRows, _] = useSelectedResources<T>(type);

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedItems={selectedRows}
      allItems={items}
      resource={type}
      actions={actions}
      standaloneActions={standaloneActions}
    />
  );
};

export const GenericActionBar = <T,>({
  selectedItems,
  allItems,
  resource,
  actions,
  standaloneActions = [],
}: ActionBarProps<T>) => {
  const { sidebarMinimized } = useLayoutContext();
  if (!selectedItems?.length) return null;

  return (
    <div
      className={cn(
        'fixed inset-x-0 bottom-0 z-20 flex min-w-0 flex-wrap items-center justify-center gap-x-4 gap-y-2 border-t border-sidebar-border bg-sidebar p-2 shadow-lg transition-[left] duration-200 ease-in-out sm:justify-between lg:min-h-[var(--layout-footer-height)]',
        sidebarMinimized ? 'lg:left-[var(--sidebar-width-icon)]' : 'lg:left-[var(--sidebar-width)]',
      )}>
      <div className="w-full text-center text-xs text-muted-foreground sm:mt-2 sm:w-auto sm:flex-1 sm:text-left">
        {selectedItems.length} of {allItems?.length} {resource.toLowerCase()}(s) selected.
      </div>
      <div className="grid w-full min-w-0 grid-cols-2 gap-2 sm:flex sm:w-auto sm:items-center">
        {standaloneActions.map((Action, id) => (
          <div className="col-span-2 flex min-w-0 [&>*]:w-full sm:col-span-1 sm:[&>*]:w-auto" key={id}>
            <Action resources={selectedItems} />
          </div>
        ))}
        <ButtonGroup className={cn('col-span-2', RESPONSIVE_BUTTON_GROUP_CLASS)}>
          {actions.map((Action, id) => (
            <Action resources={selectedItems} key={id} />
          ))}
        </ButtonGroup>
      </div>
    </div>
  );
};

export const GenericActionBarButtons = <T,>({
  actions,
  standaloneActions = [],
  resource,
}: {
  actions: ButtonActionComponent<any>[];
  standaloneActions?: ButtonActionComponent<any>[];
  resource: T;
}) => {
  return (
    <div className="grid w-full min-w-0 grid-cols-2 gap-2 sm:flex sm:w-auto sm:flex-wrap sm:items-center sm:justify-end">
      {standaloneActions.map((Action, id) => (
        <div className="col-span-2 flex min-w-0 [&>*]:w-full sm:col-span-1 sm:[&>*]:w-auto" key={id}>
          <Action resource={resource} />
        </div>
      ))}
      <ButtonGroup className={cn('col-span-2', RESPONSIVE_BUTTON_GROUP_CLASS)}>
        {actions.map((Action, id) => (
          <Action resource={resource} key={id} />
        ))}
      </ButtonGroup>
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
  resource: ResourceType;
  allItems: T[] | undefined;
  selectedItems: T[] | undefined;
  actions: ButtonGroupComponent<T>[];
  standaloneActions?: ButtonGroupComponent<T>[];
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
