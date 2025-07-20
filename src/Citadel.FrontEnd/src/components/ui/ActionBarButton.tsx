import { cn } from '@/lib/utils';

export const ActionBarButton = ({
  onClick,
  disabled,
  icon: Icon,
  label,
  className = '',
  ariaLabel,
}: {
  onClick: () => void;
  disabled: boolean;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  className?: string;
  ariaLabel?: string;
}) => (
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
