import { MoreHorizontal } from 'lucide-react';
import { Button } from '../ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { useNavigate } from 'react-router';
import { startTransition, useCallback } from 'react';

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

const DropdownActions = ({ items }: DropdownActionsProps) => (
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

interface BaseContext<T> {
  setDialogData: (data: { open: boolean; currentSelection: T[] }) => void;
}

interface CreateDropdownConfig<T> {
  resource: T;
  context: BaseContext<T>;
  actions: (args: {
    navigate: ReturnType<typeof useNavigate>;
    openDialog: () => void;
  }) => DropdownAction[];
}

export function createTableDropdown<T>({ resource, context, actions }: CreateDropdownConfig<T>) {
  const navigate = useNavigate();

  const openDialog = useCallback(() => {
    startTransition(() => {
      context.setDialogData({ open: true, currentSelection: [resource] });
    });
  }, [context, resource]);

  const items = actions({ navigate, openDialog });
  return <DropdownActions items={items} />;
}