import { notifyRequestError } from '@/lib/request-error';
import { Loader2, MoreHorizontal } from 'lucide-react';
import { Button } from '../ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { forwardRef, ReactNode, startTransition, useEffect, useRef, useState } from 'react';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog';
import { toast } from 'sonner';
import { Input } from '../ui/input';
import { ConfirmButton } from './action-with-dialog';
import React from 'react';
import { ActionData, DropdownActionComponent } from '@/pages/types';
import { useDialogHotkeys } from '@/lib/hooks';

export interface DropdownAction {
  id: string;
  label: string;
  icon: React.ReactElement;
  onClick: () => void;
  disabled?: boolean;
  danger?: boolean;
  separatorBefore?: boolean;
}

interface DropdownActionsProps {
  items: DropdownAction[];
}

export const DropdownActions = ({ items }: DropdownActionsProps) => (
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button variant="ghost" className="h-8 w-8 p-0">
        <span className="sr-only">Open menu</span>
        <MoreHorizontal className="h-4 w-4" />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
      {items.map((action) => (
        <div key={action.id}>
          {action.separatorBefore && <DropdownMenuSeparator />}
          <DropdownMenuItem
            onClick={action.onClick}
            disabled={action.disabled}
            className={cn(
              'grow rounded-sm px-3 py-2 text-[12px] font-semibold text-foreground/70',
              action.danger ? 'text-danger hover:text-danger!' : '',
            )}>
            {action.icon}
            <span>{action.label}</span>
          </DropdownMenuItem>
        </div>
      ))}
    </DropdownMenuContent>
  </DropdownMenu>
);

export const RowActionMenu = <T extends { id: string; name: string }>({
  actions,
  resource,
  onAction,
}: {
  actions: Record<string, DropdownActionComponent<T>>;
  resource: T;
  onAction?: (action: { key: string; data?: ActionData }) => void;
}) => {
  const [activeAction, setActiveAction] = useState<{
    key: string;
    data?: ActionData;
  } | null>(null);

  return (
    <>
      <RowActionDropdown
        actions={actions}
        resource={resource}
        onActionSelect={(action) => {
          if (action.key === 'confirm') {
            setActiveAction(action);
            return;
          }

          onAction?.(action);
        }}
      />
      <ActionDialog action={activeAction} onClose={() => setActiveAction(null)} />
    </>
  );
};

export const RowActionDropdown = <T,>({
  actions,
  resource,
  onActionSelect,
}: {
  actions: Record<string, DropdownActionComponent<T>>;
  resource: T;
  onActionSelect: (action: { key: string; data?: ActionData }) => void;
}) => {
  const entries = Object.entries(actions);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-38 bg-background py-2">
        {entries.map(([key, Action]) => (
          <Action
            key={key}
            resource={resource}
            onAction={(actionKey, data) => onActionSelect({ key: actionKey, data })}
          />
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

const ActionDialog = ({
  action,
  onClose,
}: {
  action: { key: string; data?: ActionData } | null;
  onClose: () => void;
}) => {
  const [input, setInput] = useState('');
  const [loading, setIsLoading] = useState(false);
  const confirmButtonRef = useRef<HTMLButtonElement>(null);

  const hasData = !!action?.data;
  const isConfirmDisabled = action?.data ? action.data.disabled || action.data.name !== input || loading : true;

  const handleConfirm = () => {
    if (!action?.data) return;
    try {
      setIsLoading(true);
      const maybePromise = action.data.onClick?.();
      Promise.resolve(maybePromise)
        .then(() => onClose())
        .catch(notifyRequestError)
        .finally(() => setIsLoading(false));
    } catch (error) {
      notifyRequestError(error);
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (hasData) {
      setInput('');
      setIsLoading(false);
    }
  }, [hasData]);

  useDialogHotkeys({
    enabled: hasData,
    onConfirm: handleConfirm,
    onCancel: onClose,
    confirmDisabled: isConfirmDisabled,
    confirmButtonRef,
  });

  if (!hasData) return null;

  const { name, title, icon, variant, description } = action.data!;

  return (
    <Dialog open={hasData} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Confirm {title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        <div className="flex flex-col gap-4 my-4">
          <p
            onClick={() => {
              navigator.clipboard.writeText(name);
              toast(`Copied "${name}" to clipboard!`);
            }}
            className="cursor-pointer break-all">
            Please enter <b>{name}</b> below to confirm this action.
            <br />
            <span className="text-xs text-muted-foreground">You may click the name in bold to copy it</span>
          </p>
          <Input value={input} onChange={(e) => setInput(e.target.value)} className="focus-visible:ring-1" autoFocus />
        </div>
        <DialogFooter>
          <ConfirmButton
            ref={confirmButtonRef}
            title={title}
            icon={icon}
            disabled={isConfirmDisabled}
            loading={loading}
            onClick={handleConfirm}
            variant={variant}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export const DropdownActionButton = forwardRef<
  HTMLDivElement,
  {
    title: string;
    icon: ReactNode;
    iconPosition?: 'left' | 'right';
    disabled?: boolean;
    className?: string;
    onClick?: () => void;
    loading?: boolean;
    inset?: boolean;
    separatorBefore?: boolean;
    variant?: 'default' | 'destructive' | 'link' | 'outline' | 'secondary' | 'ghost' | null | undefined;
  }
>(
  (
    {
      title,
      icon,
      iconPosition = 'left',
      disabled,
      className,
      loading,
      onClick,
      inset,
      separatorBefore,
      variant = 'default',
    },
    ref,
  ) => {
    const iconClasses = cn('w-4 h-4', loading && 'animate-spin', variant === 'destructive' && 'text-destructive');

    const renderIcon = loading ? (
      <Loader2 className={iconClasses} />
    ) : React.isValidElement(icon) ? (
      React.cloneElement(icon as React.ReactElement<React.SVGProps<SVGSVGElement>>, {
        className: cn((icon.props as any).className, iconClasses),
      })
    ) : (
      icon
    );

    const handleClick = () => {
      startTransition(() => onClick?.());
    };

    return (
      <>
        {separatorBefore && <DropdownMenuSeparator />}
        <DropdownMenuItem
          variant={variant === 'destructive' ? 'destructive' : 'default'}
          inset={inset}
          className={cn(
            'flex items-center gap-3 cursor-pointer px-3 py-2 text-sm',
            disabled && 'opacity-50 cursor-not-allowed',
            className,
          )}
          onSelect={handleClick}
          disabled={disabled || loading}
          ref={ref}>
          {iconPosition === 'left' && renderIcon}
          <span>{title}</span>
          {iconPosition === 'right' && renderIcon}
        </DropdownMenuItem>
      </>
    );
  },
);
