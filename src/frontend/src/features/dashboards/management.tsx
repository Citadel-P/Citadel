import { useState } from 'react';
import { useNavigate } from 'react-router';
import { Copy, Share2, Trash } from 'lucide-react';
import type { DashboardConfiguration, DashboardSummaryView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { SetSelectedHome } from './actions';
import { DashboardDeleteDialog } from './delete-dialog';
import { useCanManageSharing, useSharingDisabled, SharingDialog } from './sharing';
import { CreateDashboardDialog } from './create';
import { useCanManageDashboards } from './access';

export function DuplicateDashboard({ name, configuration }: { name: string; configuration: DashboardConfiguration }) {
  const [open, setOpen] = useState(false);
  const canManage = useCanManageDashboards();
  if (!canManage) return null;
  return (
    <>
      <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
        <Copy /> Duplicate
      </Button>
      {open && <CreateDashboardDialog copy={{ name, configuration }} close={() => setOpen(false)} />}
    </>
  );
}

export function DashboardActions({ board }: { board: DashboardSummaryView }) {
  const [deleting, setDeleting] = useState(false);
  const [sharing, setSharing] = useState<DashboardSummaryView>();
  const canShare = useCanManageSharing(board);
  const sharingDisabled = useSharingDisabled(board);
  const navigate = useNavigate();
  return (
    <>
      <SetSelectedHome resources={[board]} />
      {canShare && (
        <Button variant="outline" disabled={sharingDisabled} onClick={() => setSharing(board)}>
          <Share2 /> Sharing
        </Button>
      )}
      {board.canDelete && (
        <Button variant="destructive" onClick={() => setDeleting(true)}>
          <Trash /> Delete
        </Button>
      )}
      {sharing && canShare && <SharingDialog board={sharing} close={() => setSharing(undefined)} />}
      {deleting && board.canDelete && (
        <DashboardDeleteDialog
          boards={[board]}
          close={() => setDeleting(false)}
          onDeleted={() => navigate('/dashboards', { replace: true })}
        />
      )}
    </>
  );
}
