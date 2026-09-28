import { forwardRef, ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '../ui/dialog';
import { toast } from 'sonner';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { cn } from '@/lib/utils';
import { Loader2 } from 'lucide-react';
import { useSelectedResources } from '@/lib/atoms';
import { ResourceType } from '@/api/types';
import { useConfirmByName, useDialogHotkeys } from '@/lib/hooks';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip';

export const ActionButton = forwardRef<
  HTMLButtonElement,
  {
    variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
    size?: 'default' | 'sm' | 'lg' | 'icon' | null | undefined;
    title: string;
    icon: ReactNode;
    iconPosition?: 'left' | 'right';
    disabled?: boolean;
    className?: string;
    onClick?: React.MouseEventHandler<HTMLButtonElement>;
    onBlur?: React.FocusEventHandler<HTMLButtonElement>;
    loading?: boolean;
    disabledReason?: string;
  }
>(
  (
    {
      variant,
      size,
      title,
      icon,
      iconPosition = 'right',
      disabled,
      disabledReason,
      className,
      loading,
      onClick,
      onBlur,
    },
    ref,
  ) => {
    const renderIcon = loading ? <Loader2 className="w-4 h-4 animate-spin" /> : icon;

    const button = (
      <Button
        size={size}
        variant={variant || 'secondary'}
        className={cn(
          'flex flex-1 shrink-0 items-center justify-between gap-2 rounded-sm text-xs max-w-47.5',
          className,
        )}
        onClick={onClick}
        onBlur={onBlur}
        disabled={disabled || loading}
        ref={ref}>
        {iconPosition === 'left' && renderIcon}
        <span>{title}</span>
        {iconPosition === 'right' && renderIcon}
      </Button>
    );

    if (!disabled || !disabledReason) return button;

    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <span className="inline-flex min-w-max flex-1 cursor-not-allowed" tabIndex={0}>
            {button}
          </span>
        </TooltipTrigger>
        <TooltipContent>{disabledReason}</TooltipContent>
      </Tooltip>
    );
  },
);

export const ConfirmButton = forwardRef<
  HTMLButtonElement,
  {
    variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
    size?: 'default' | 'sm' | 'lg' | 'icon' | null | undefined;
    title: string;
    icon: ReactNode;
    onClick?: React.MouseEventHandler<HTMLButtonElement>;
    loading?: boolean;
    disabled?: boolean;
    className?: string;
  }
>(({ variant, size, title, icon, disabled, loading, onClick, className }, ref) => {
  return (
    <ActionButton
      ref={ref}
      variant={variant}
      size={size}
      title={title}
      icon={icon}
      disabled={disabled}
      onClick={(e) => {
        e.stopPropagation();
        onClick && onClick(e);
      }}
      loading={loading}
      className={className}
    />
  );
});

export const ActionWithDialog = ({
  name,
  title,
  icon,
  iconPosition,
  disabled,
  disabledReason,
  onClick,
  additional,
  targetClassName,
  variant,
  open: controlledOpen,
  onOpenChange,
  renderTrigger = true,
  description,
}: {
  name: string;
  title: string;
  icon: ReactNode;
  iconPosition?: 'left' | 'right';
  disabled?: boolean;
  disabledReason?: string;
  onClick?: () => void | Promise<unknown>;
  additional?: ReactNode;
  targetClassName?: string;
  variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  renderTrigger?: boolean;
  description?: ReactNode;
}) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const open = controlledOpen ?? internalOpen;
  const setOpen = useCallback(
    (nextOpen: boolean) => {
      if (controlledOpen === undefined) setInternalOpen(nextOpen);
      onOpenChange?.(nextOpen);
    },
    [controlledOpen, onOpenChange],
  );
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  const { input, setInput, isLoading, isConfirmDisabled, handleConfirm } = useConfirmByName({
    name,
    disabled,
    onConfirm: onClick,
    onClose: () => setOpen(false),
    hotkeysEnabled: open,
  });

  useDialogHotkeys({
    enabled: open,
    onConfirm: handleConfirm,
    onCancel: () => setOpen(false),
    confirmDisabled: isConfirmDisabled,
    confirmButtonRef,
  });
  useEffect(() => {
    if (open) setInput('');
  }, [open, setInput]);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      {renderTrigger && (
        <DialogTrigger asChild>
          <ActionButton
            className={targetClassName}
            title={title}
            icon={icon}
            iconPosition={iconPosition}
            disabled={disabled}
            disabledReason={disabledReason}
            onClick={() => setOpen(true)}
            loading={isLoading}
            variant={variant}
          />
        </DialogTrigger>
      )}
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
          <Input
            aria-label={`Enter ${name} to confirm`}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            className="focus-visible:ring-1"
          />
          {additional}
        </div>
        <DialogFooter>
          <ConfirmButton
            ref={confirmButtonRef}
            title={title}
            icon={icon}
            disabled={isConfirmDisabled}
            variant={variant}
            onClick={handleConfirm}
            loading={isLoading}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export const ActionGroup = <T extends { id: string; name: string }>({
  actions,
  resource,
}: {
  actions: Record<string, React.ComponentType<{ resource: T; targetClassName?: string }>>;
  resource: T;
}) => {
  const entries = Object.entries(actions);

  return (
    <div className="inline-flex overflow-hidden border border-border rounded-lg">
      {entries.map(([key, Action]) => {
        return (
          <div key={key} className={cn('border-r border-border last:border-r-0')}>
            <Action resource={resource} />
          </div>
        );
      })}
    </div>
  );
};

export const GroupActionWithDialog = <T extends { id: string; name: string }>({
  name,
  type,
  title,
  icon,
  iconPosition,
  disabled,
  disabledReason,
  onClick,
  additional,
  targetClassName,
  variant,
  description,
}: {
  name: string;
  title: string;
  type: ResourceType;
  icon: ReactNode;
  iconPosition?: 'left' | 'right';
  disabled?: boolean;
  disabledReason?: string;
  onClick?: () => void | Promise<unknown>;
  additional?: ReactNode;
  targetClassName?: string;
  variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
  description?: ReactNode;
}) => {
  const [open, setOpen] = useState(false);
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  const [selected, _] = useSelectedResources<T>(type);
  const { input, setInput, isLoading, isConfirmDisabled, handleConfirm, reset } = useConfirmByName({
    name,
    disabled,
    onConfirm: onClick,
    onClose: () => setOpen(false),
    hotkeysEnabled: open,
  });
  useDialogHotkeys({
    enabled: open,
    onConfirm: handleConfirm,
    onCancel: () => setOpen(false),
    confirmDisabled: isConfirmDisabled,
    confirmButtonRef,
  });
  return (
    <Dialog
      open={open}
      onOpenChange={(open) => {
        setOpen(open);
        reset();
      }}>
      <DialogTrigger asChild>
        <ActionButton
          className={targetClassName}
          title={title}
          icon={icon}
          iconPosition={iconPosition}
          disabled={disabled}
          disabledReason={disabledReason}
          onClick={() => setOpen(true)}
          loading={isLoading}
          variant={variant}
        />
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Group Execute - {title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        <div className="flex flex-col gap-4 my-4 whitespace-nowrap overflow-x-auto">
          <ul className="p-4 bg-accent text-sm list-disc list-inside max-h-75 overflow-y-auto">
            {selected.map((resource, i) => (
              <li key={i}>{resource.name}</li>
            ))}
          </ul>
          <p
            onClick={() => {
              navigator.clipboard.writeText(name);
              toast(`Copied "${name}" to clipboard!`);
            }}
            className="cursor-pointer">
            Please enter <b>{name}</b> below to confirm this action.
            <br />
            <span className="text-xs text-muted-foreground">You may click the name in bold to copy it</span>
          </p>
          <div className="p-1">
            <Input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              className="focus-visible:ring-1 shadow-xs"
            />
            {additional}
          </div>
        </div>
        <DialogFooter>
          <ConfirmButton
            ref={confirmButtonRef}
            title={title}
            icon={icon}
            variant={variant}
            disabled={isConfirmDisabled}
            loading={isLoading}
            onClick={handleConfirm}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
