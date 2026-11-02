import { cloneElement, type ReactElement, type ReactNode } from 'react';
import type { AvailableUpdate } from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Link } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';

export function useAvailableUpdate() {
  const { applicationInfo } = useAppContext();
  const { data: profile } = useRead('getCurrentProfile');
  return profile?.data.authorization.isAdministrator ? applicationInfo?.availableUpdate : undefined;
}

export function UpdateDetails({ children }: { update: AvailableUpdate; children: ReactElement }) {
  const button = children as ReactElement<{ asChild?: boolean; children?: ReactNode }>;
  return cloneElement(button, { asChild: true }, <Link to="/updates">{button.props.children}</Link>);
}

export function UpdateNotice() {
  const update = useAvailableUpdate();
  if (!update) return null;
  return (
    <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 pt-4 sm:px-6">
      <Alert className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <AlertTitle>New version available</AlertTitle>
          <AlertDescription>Citadel {update.version} is available.</AlertDescription>
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
