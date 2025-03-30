import { cn } from '@/lib/utils';
import { DropdownMenuItem } from './dropdown-menu';

// Reusable ActionMenuItem component
export const ActionMenuItem: React.FC<{
  onClick: () => void;
  disabled?: boolean;
  icon: React.ReactNode;
  label: string;
  className?: string;
}> = ({ onClick, disabled, icon, label, className }) => (
  <DropdownMenuItem
    onClick={onClick}
    disabled={disabled ?? false}
    className={cn('grow rounded-sm px-3 py-2 text-[12px] font-semibold text-foreground/70', className)}>
    {icon}
    <span className={className}>{label}</span>
  </DropdownMenuItem>
);
