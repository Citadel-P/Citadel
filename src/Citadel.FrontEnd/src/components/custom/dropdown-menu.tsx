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
import { DockerResourceType } from '@/api/types';
import { useDeleteDialog } from '@/lib/hooks';

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

interface CreateDropdownConfig<T> {
  type: DockerResourceType;
  actions: (args: {
    navigate: ReturnType<typeof useNavigate>;
    openDialog: (targets: T | T[]) => void;
  }) => DropdownAction[];
}

export function createTableDropdown<T>({ type, actions }: CreateDropdownConfig<T>) {
  const navigate = useNavigate();
  const { openDialog } = useDeleteDialog<T>({ type });

  const handleOpenDialog = useCallback(
    (targets: T | T[]) => {
      startTransition(() => openDialog(targets));
    },
    [openDialog],
  );

  const items = actions({ navigate, openDialog: handleOpenDialog });

  return <DropdownActions items={items} />;
}
