import { PreferenceDialog } from './home-preference';
import { useCanManageSharing, SharingItem, SharingDialog } from './sharing';
import { MoreHorizontal } from 'lucide-react';
import { useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import type { DashboardConfiguration, DashboardSummaryView } from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useMutate } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
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
  const [version, setVersion] = useState(board.revision);
  const client = useQueryClient();
  const navigate = useNavigate();
  const remove = useMutate('deleteDashboard', {
    onSuccess: () => {
      navigate('/dashboards', { replace: true });
      void client.invalidateQueries({ queryKey: ['listDashboards'] });
      void client.invalidateQueries({ queryKey: ['getDashboardDefault'] });
      client.removeQueries({ queryKey: resourceQueryKey('getDashboard', { id: board.id }), exact: true });
    },
  });
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
            <DropdownMenuItem
              onSelect={() => {
                setVersion(board.revision);
                setAction('delete');
              }}>
              Delete dashboard
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
      {action === 'home' && <PreferenceDialog board={board} close={() => setAction(undefined)} />}
      {sharing && canShare && <SharingDialog board={sharing} close={() => setSharing(undefined)} />}
      {action === 'duplicate' && configuration && (
        <CreateDashboardDialog copy={{ name: board.name, configuration }} close={() => setAction(undefined)} />
      )}
      {action === 'delete' && board.canDelete && (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open && !remove.isPending) setAction(undefined);
          }}>
          <DialogContent>
            <DialogTitle>Delete {board.name}?</DialogTitle>
            <DialogDescription>
              This removes the dashboard and its home preferences. Resources shown on it are kept.
            </DialogDescription>
            {remove.isError && <p role="alert">{requestErrorMessage(remove.error)}</p>}
            <DialogFooter>
              <Button variant="outline" disabled={remove.isPending} onClick={() => setAction(undefined)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                disabled={remove.isPending}
                onClick={() => remove.mutate({ id: board.id, query: { revision: version } })}>
                Delete dashboard
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
