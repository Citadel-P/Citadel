import { useContext, useState } from 'react';
import { Link } from 'react-router';
import { TargetCell } from '@/components/custom/common';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useRead } from '@/lib/hooks';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { ViewOnlyContext } from './tile';

export function CopyResourceLink({ path }: { path: string }) {
  const [copied, copy] = useCopyToClipboard(2000);
  return (
    <Button size="sm" variant="ghost" onClick={() => copy(new URL(path, window.location.origin).href)}>
      {copied ? 'Copied' : 'Copy link'}
    </Button>
  );
}

export function ActivityActions({ id }: { id: string }) {
  const [open, setOpen] = useState(false);
  const paused = useContext(ViewOnlyContext);
  return (
    <Popover open={open && !paused} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button disabled={paused} size="sm" variant="ghost" aria-label="Activity links">
          Links
        </Button>
      </PopoverTrigger>
      <PopoverContent>{open && !paused && <ActivityTarget id={id} />}</PopoverContent>
    </Popover>
  );
}

function ActivityTarget({ id }: { id: string }) {
  const result = useRead('getActivity', { id }, { retry: false });
  const activity = result.data?.data;
  if (!activity || result.isError) return <p>{result.isLoading ? 'Loading…' : 'Target unavailable.'}</p>;
  const { resourceType, resourceId, resourceName, info } = activity;
  const prefix = { BackupPolicy: 'backup-policies', Script: 'scripts', Build: 'builds' }[
    resourceType as 'BackupPolicy' | 'Script' | 'Build'
  ];
  return (
    <div className="space-y-2 text-sm">
      <Link className="table-link" to={'/activities/' + id}>
        Activity details
      </Link>
      {resourceId && <TargetCell resourceType={resourceType} resourceId={resourceId} resourceName={resourceName} />}
      {prefix && resourceId && 'runId' in info && info.runId && (
        <Link
          className="table-link block"
          to={
            '/' +
            prefix +
            '/edit/' +
            encodeURIComponent(resourceId) +
            '?runId=' +
            encodeURIComponent(info.runId) +
            '#runs'
          }>
          View run
        </Link>
      )}
    </div>
  );
}
