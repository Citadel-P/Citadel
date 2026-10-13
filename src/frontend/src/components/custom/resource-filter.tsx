import { ComponentProps, ComponentType, ReactNode } from 'react';
import { Minus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Command, CommandEmpty, CommandInput, CommandList } from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { cn } from '@/lib/utils';

export function ResourceFilterChip({
  children,
  label,
  className,
  icon,
  ...props
}: Omit<ComponentProps<'button'>, 'aria-label'> & { label: string; icon?: ReactNode }) {
  return (
    <button
      {...props}
      type="button"
      title={label}
      aria-label={label}
      className={cn(
        'inline-flex h-(--control-height) max-w-48 items-center gap-1.5 rounded-sm border border-border px-2 text-xs font-medium shadow-xs',
        className,
      )}>
      <Minus className="size-3 shrink-0" />
      {icon}
      <span className="truncate">{children}</span>
    </button>
  );
}

export function ResourceFilterDropdown({
  label,
  icon: Icon,
  disabled,
  searchPlaceholder,
  emptyMessage,
  filter,
  open,
  onOpenChange,
  contentClassName,
  children,
  footer,
}: {
  label: string;
  icon: ComponentType<{ className?: string }>;
  disabled?: boolean;
  searchPlaceholder: string;
  emptyMessage: string;
  filter?: ComponentProps<typeof Command>['filter'];
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  contentClassName?: string;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          className="h-(--control-height) shrink-0 rounded-sm px-2.5"
          disabled={disabled}>
          <Icon className="size-3.5" />
          {label}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className={cn('w-72 p-0 bg-background', contentClassName)}>
        <Command filter={filter}>
          <CommandInput placeholder={searchPlaceholder} />
          <CommandList className="max-h-72 overscroll-contain" onWheel={(event) => event.stopPropagation()}>
            <CommandEmpty>{emptyMessage}</CommandEmpty>
            {children}
          </CommandList>
        </Command>
        {footer}
      </PopoverContent>
    </Popover>
  );
}
