import { usePromptHotkeys } from '@/lib/hooks';
import { forwardRef, ReactNode, useRef, useState } from 'react';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog';
import { toast } from 'sonner';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { cn } from '@/lib/utils';
import { Loader2 } from 'lucide-react';

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
  }
>(({ variant, size, title, icon, iconPosition = 'right', disabled, className, loading, onClick, onBlur }, ref) => {
  const renderIcon = loading ? <Loader2 className="w-4 h-4 animate-spin" /> : icon;

  return (
    <Button
      size={size}
      variant={variant || 'secondary'}
      className={cn(
        'flex flex-1 shrink-0 items-center justify-between gap-2 rounded-sm text-xs max-w-[190px]',
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
});

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
  loading,
  onClick,
  additional,
  targetClassName,
  variant,
}: {
  name: string;
  title: string;
  icon: ReactNode;
  iconPosition?: 'left' | 'right';
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void;
  additional?: ReactNode;
  targetClassName?: string;
  variant?: 'link' | 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | null | undefined;
}) => {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState('');
  const confirmButtonRef = useRef<HTMLButtonElement>(null);

  usePromptHotkeys({
    onConfirm: () => {
      if (name === input && !disabled) {
        onClick && onClick();
        setOpen(false);
      }
    },
    onCancel: () => setOpen(false),
    enabled: open,
    confirmDisabled: disabled || name !== input,
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(open) => {
        setOpen(open);
        setInput('');
      }}>
      <DialogTrigger asChild>
        <ActionButton
          className={targetClassName}
          title={title}
          icon={icon}
          iconPosition={iconPosition}
          disabled={disabled}
          onClick={() => setOpen(true)}
          loading={loading}
          variant={variant}
        />
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Confirm {title}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4 my-4">
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
          <Input value={input} onChange={(e) => setInput(e.target.value)} className="focus-visible:ring-1" />
          {additional}
        </div>
        <DialogFooter>
          <ConfirmButton
            ref={confirmButtonRef}
            title={title}
            icon={icon}
            disabled={disabled || name !== input}
            onClick={() => {
              onClick && onClick();
              setOpen(false);
            }}
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
