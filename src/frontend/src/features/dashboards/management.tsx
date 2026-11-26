import { DashboardDeleteDialog } from './delete-dialog';
import { PreferenceDialog } from './home-preference';
import { useCanManageSharing, SharingItem, SharingDialog } from './sharing';
import { MoreHorizontal } from 'lucide-react';
import { useState } from 'react';
import { Link, useNavigate } from 'react-router';
import type { DashboardConfiguration, DashboardSummaryView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { CreateDashboardDialog } from './create';

export function DashboardMenu({
  board,
  configuration,
}: {
  board: DashboardSummaryView;
  configuration?: DashboardConfiguration;
}) {
  const [action, setAction] = useState<'duplicate' | 'delete' | 'home'>();
  const canShare = useCanManageSharing(board);
  const [sharing, setSharing] = useState<DashboardSummaryView>();
  const navigate = useNavigate();
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="icon" aria-label="Dashboard options">
            <MoreHorizontal />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem asChild>
            <Link to="/dashboards">Choose a dashboard</Link>
          </DropdownMenuItem>
          {configuration && (
            <DropdownMenuItem onSelect={() => setAction('duplicate')}>Duplicate dashboard</DropdownMenuItem>
          )}
          <DropdownMenuItem onSelect={() => setAction('home')}>Set as home</DropdownMenuItem>
          {canShare && <SharingItem board={board} select={() => setSharing(board)} />}
          {board.canDelete && (
            <DropdownMenuItem onSelect={() => setAction('delete')}>Delete dashboard</DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
      {action === 'home' && <PreferenceDialog board={board} close={() => setAction(undefined)} />}
      {sharing && canShare && <SharingDialog board={sharing} close={() => setSharing(undefined)} />}
      {action === 'duplicate' && configuration && (
        <CreateDashboardDialog copy={{ name: board.name, configuration }} close={() => setAction(undefined)} />
      )}
      {action === 'delete' && board.canDelete && (
        <DashboardDeleteDialog
          boards={[board]}
          close={() => setAction(undefined)}
          onDeleted={() => navigate('/dashboards', { replace: true })}
        />
      )}
    </>
  );
}
