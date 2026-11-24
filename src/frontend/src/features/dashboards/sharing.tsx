import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  DashboardSharingMode,
  DashboardVisibility,
  LicenseCapability,
  type DashboardSummaryView,
} from '@/api/generated/api.types';
import { resourceQueryKey } from '@/api/dispatch';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { useMutate, useRead } from '@/lib/hooks';
import { requestErrorMessage, requestErrorStatus } from '@/lib/request-error';

export function DashboardSharing({ board }: { board: DashboardSummaryView }) {
  const profile = useRead('getCurrentProfile');
  if (
    profile.isError ||
    !profile.data?.data.authorization.isAdministrator ||
    board.visibility === DashboardVisibility.Private
  )
    return null;
  return <SharingButton board={board} />;
}

function SharingButton({ board }: { board: DashboardSummaryView }) {
  const { hasCapability } = useLicenseEntitlements();
  const disabled =
    board.visibility === DashboardVisibility.InstanceShared && !hasCapability(LicenseCapability.CustomAccessControl);
  // Confirm the revision and audience seen when the administrator opened the dialog.
  const [selection, setSelection] = useState<DashboardSummaryView>();
  return (
    <>
      <Button
        variant="outline"
        disabled={disabled}
        title={disabled ? 'Restricting access requires Team' : undefined}
        onClick={() => setSelection(board)}>
        Sharing
      </Button>
      {selection && <SharingDialog board={selection} close={() => setSelection(undefined)} />}
    </>
  );
}

function SharingDialog({ board, close }: { board: DashboardSummaryView; close: () => void }) {
  const client = useQueryClient();
  const restrict = board.visibility === DashboardVisibility.InstanceShared;
  const change = useMutate('setDashboardVisibility', {
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['listDashboards'] });
      void client.invalidateQueries({ queryKey: resourceQueryKey('getDashboard', { id: board.id }) });
      void client.invalidateQueries({ queryKey: ['getDashboardDefault'] });
      close();
    },
    onError: (error) => {
      if (requestErrorStatus(error) === 409) {
        void client.invalidateQueries({ queryKey: resourceQueryKey('getDashboard', { id: board.id }) });
        void client.invalidateQueries({ queryKey: ['listDashboards'] });
      }
    },
  });
  const conflict = requestErrorStatus(change.error) === 409;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !change.isPending) close();
      }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {restrict ? 'Restrict access' : 'Share with everyone'} · {board.name}
          </DialogTitle>
          <DialogDescription>
            {restrict
              ? 'Only administrators and people with Dashboard access through their user, team or role will be able to open this dashboard.'
              : 'Every signed-in user will be able to read this dashboard, including its saved notes and links. Their access to the resources it shows stays the same.'}
          </DialogDescription>
        </DialogHeader>
        {change.isError && (
          <p role="alert">
            {conflict
              ? 'The dashboard has changed. Close this dialog and review its current access.'
              : requestErrorMessage(change.error)}
          </p>
        )}
        <DialogFooter>
          <Button variant="outline" disabled={change.isPending} onClick={close}>
            Cancel
          </Button>
          <Button
            disabled={change.isPending || conflict}
            onClick={() =>
              change.mutate({
                id: board.id,
                data: {
                  revision: board.revision,
                  visibility: restrict ? DashboardSharingMode.Restricted : DashboardSharingMode.InstanceShared,
                },
              })
            }>
            {restrict ? 'Restrict access' : 'Share with everyone'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
