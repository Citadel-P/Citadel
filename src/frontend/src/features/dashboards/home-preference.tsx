import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { DashboardVisibility, type DashboardSummaryView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { useMutate, useRead } from '@/lib/hooks';
import { requestErrorMessage } from '@/lib/request-error';
import { useCanManageDashboards } from './access';

// The API resolves the personal preference, then the visible instance default.
export function useDashboardHomeAction(board?: DashboardSummaryView) {
  const canSet = useCanManageDashboards();
  const home = useRead('getDashboardDefault', undefined, { enabled: !!board, staleTime: 30_000 });
  const current = !!board && !home.isError && home.data?.data.dashboardId === board.id;
  return {
    current,
    canSet,
    label: current ? 'Current home' : 'Set as home',
    disabled: current && board.visibility === DashboardVisibility.Private,
  };
}

export function DashboardHomePreference({ board }: { board?: DashboardSummaryView }) {
  const [open, setOpen] = useState(false);
  const home = useDashboardHomeAction(board);
  if (!home.canSet) return null;
  return (
    <>
      <Button variant="outline" disabled={home.disabled} onClick={() => setOpen(true)}>
        {board ? home.label : 'Reset home'}
      </Button>
      {open && <PreferenceDialog board={board} close={() => setOpen(false)} />}
    </>
  );
}

export function PreferenceDialog({ board, close }: { board?: DashboardSummaryView; close: () => void }) {
  const client = useQueryClient();
  const profile = useRead('getCurrentProfile');
  const canSetInstance =
    !profile.isError &&
    profile.data?.data.authorization.isAdministrator &&
    board?.visibility !== DashboardVisibility.Private;
  const [scope, setScope] = useState('personal');
  const instance = scope === 'instance';
  const onSuccess = () => {
    void client.invalidateQueries({ queryKey: ['getDashboardDefault'] });
    toast.success(instance ? 'Instance home updated.' : 'Your home updated.');
    close();
  };
  const personal = useMutate('setPersonalDashboardDefault', { onSuccess });
  const shared = useMutate('setInstanceDashboardDefault', { onSuccess });
  const mutation = instance ? shared : personal;
  const pending = shared.isPending || personal.isPending;
  const description = instance
    ? board
      ? 'People who can read this dashboard will see it on Home unless they chose their own dashboard.'
      : 'Clear the instance default. People who chose their own dashboard keep it.'
    : board
      ? 'Open this dashboard whenever you go to Home.'
      : 'Use the instance default for your home if you can read it.';
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !pending) close();
      }}>
      <DialogContent showCloseButton={!pending}>
        <DialogHeader>
          <DialogTitle>{board ? 'Set ' + board.name + ' as home' : 'Reset home dashboard'}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        {canSetInstance && (
          <RadioGroup aria-label="Apply to" value={scope} disabled={pending} onValueChange={setScope}>
            <Label className="flex items-center gap-2">
              <RadioGroupItem value="personal" /> My home
            </Label>
            <Label className="flex items-center gap-2">
              <RadioGroupItem value="instance" /> Instance default
            </Label>
          </RadioGroup>
        )}
        {mutation.isError && (
          <p role="alert" className="text-sm text-destructive">
            {requestErrorMessage(mutation.error)}
          </p>
        )}
        <DialogFooter>
          <Button variant="outline" disabled={pending} onClick={close}>
            Cancel
          </Button>
          <Button
            disabled={pending || (instance && !canSetInstance)}
            onClick={() => mutation.mutate({ data: { dashboardId: board?.id ?? null } })}>
            {instance ? 'Save instance default' : 'Save my home'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
