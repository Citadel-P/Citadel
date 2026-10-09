import type { ReactElement } from 'react';
import type { AvailableUpdate } from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';

const UPGRADE_GUIDE = 'https://docs.citadelplane.com/docs/operations/upgrade-and-rollback/';

export function useAvailableUpdate() {
  const { applicationInfo } = useAppContext();
  const { data: profile } = useRead('getCurrentProfile');
  return profile?.data.authorization.isAdministrator ? applicationInfo?.availableUpdate : undefined;
}

export function UpdateDetails({ update, children }: { update: AvailableUpdate; children: ReactElement }) {
  const { applicationInfo } = useAppContext();
  return (
    <Dialog>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Citadel {update.version} available</DialogTitle>
          <DialogDescription>
            You are running Citadel {applicationInfo?.version}. Review the release notes and back up your installation
            before upgrading. Updates are installed manually.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" asChild>
            <a href={update.releaseUrl} target="_blank" rel="noreferrer">
              Release notes
            </a>
          </Button>
          <Button asChild>
            <a href={UPGRADE_GUIDE} target="_blank" rel="noreferrer">
              Upgrade guide
            </a>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function UpdateNotice() {
  const update = useAvailableUpdate();
  if (!update) return null;
  return (
    <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 pt-4 sm:px-6">
      <Alert className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <AlertTitle>New version available</AlertTitle>
          <AlertDescription>Citadel {update.version} is ready to install.</AlertDescription>
        </div>
        <UpdateDetails update={update}>
          <Button size="sm" variant="outline">
            View update
          </Button>
        </UpdateDetails>
      </Alert>
    </div>
  );
}
