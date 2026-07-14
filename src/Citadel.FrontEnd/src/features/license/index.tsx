import { LicenseLimitView, LicenseStatus } from '@/api/generated/api.types';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { Textarea } from '@/components/ui/textarea';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { cn } from '@/lib/utils';
import { useRead, useMutate } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useQueryClient } from '@tanstack/react-query';
import { LICENSE_LIMIT_LABELS, LICENSE_STATUS_LABELS } from './license-labels';
import {
  AlertTriangle,
  CheckCheck,
  Clipboard,
  KeyRound,
  Loader2,
  RotateCcw,
  ShieldCheck,
  Trash2,
} from 'lucide-react';
import { ReactNode, useMemo, useState } from 'react';
import { toast } from 'sonner';

const ACTIVE_STATUSES = new Set<LicenseStatus>([
  LicenseStatus.Community,
  LicenseStatus.Valid,
  LicenseStatus.GracePeriod,
  LicenseStatus.NotYetValid,
]);

export default function LicensePage() {
  const queryClient = useQueryClient();
  const formatDate = useProfileDateTimeFormatter();
  const licenseQuery = useRead('getLicense');
  const requestQuery = useRead('getLicenseRequest');
  const installLicense = useMutate('installLicense');
  const removeLicense = useMutate('removeLicense');
  const [licenseInput, setLicenseInput] = useState('');
  const [confirmRemoveOpen, setConfirmRemoveOpen] = useState(false);
  const [copiedRequest, copyRequest] = useCopyToClipboard(2500);

  const license = licenseQuery.data?.data;
  const request = requestQuery.data?.data;

  const requestJson = useMemo(() => {
    if (!request) return '';
    return JSON.stringify(request, null, 2);
  }, [request]);

  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['getLicense'] }),
      queryClient.invalidateQueries({ queryKey: ['getLicenseRequest'] }),
    ]);
  };

  const handleInstall = async () => {
    const value = licenseInput.trim();
    if (!value) return;

    await installLicense.mutateAsync({ data: { license: value } });
    setLicenseInput('');
    await refresh();
    toast.success('License installed.');
  };

  const handleRemove = async () => {
    await removeLicense.mutateAsync({});
    setConfirmRemoveOpen(false);
    await refresh();
    toast.success('License removed.');
  };

  if (licenseQuery.isLoading || requestQuery.isLoading) {
    return (
      <div className="mx-auto px-4 py-4 sm:px-6 lg:container">
        <div className="rounded-sm border bg-background p-4">
          <Loader />
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto px-4 py-4 sm:px-6 lg:container">
      <div className="space-y-4 bg-background p-4">
        <header className="rounded-sm border bg-background p-4">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="min-w-0">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <div className="flex size-10 items-center justify-center rounded-sm bg-primary/10 text-primary">
                  <KeyRound className="size-5" />
                </div>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h1 className="truncate text-lg font-semibold text-foreground">License</h1>
                    {license && <LicenseStatusBadge status={license.status} />}
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Offline license state and quota usage for this Citadel instance.
                  </p>
                </div>
              </div>
            </div>

            {license && (
              <div className="grid gap-3 text-sm sm:grid-cols-2 lg:min-w-[520px] lg:grid-cols-4">
                <HeaderFact label="Edition" value={license.edition} />
                <HeaderFact label="Instance" value={shortId(license.instanceId)} />
                <HeaderFact label="Expires" value={license.expiresAt ? formatDate(license.expiresAt) : '-'} />
                <HeaderFact label="Fingerprint" value={license.fingerprint ? shortFingerprint(license.fingerprint) : '-'} />
                {license.replacedLicenseId ? <HeaderFact label="Replaces" value={license.replacedLicenseId} /> : null}
              </div>
            )}
          </div>
        </header>

        {license?.warnings?.length ? (
          <Alert className="rounded-sm border-amber-500/40 bg-amber-500/5 text-amber-700 dark:text-amber-300">
            <AlertTriangle className="size-4" />
            <AlertTitle>License needs attention</AlertTitle>
            <AlertDescription>{license.warnings.join(' ')}</AlertDescription>
          </Alert>
        ) : null}

        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
          <section className="rounded-sm border bg-background">
            <SectionHeader
              title="Quota Usage"
              description="Current usage is measured from the database when this page loads."
            />
            <div className="divide-y">
              {(license?.limits ?? []).map((limit) => (
                <QuotaRow key={limit.limit} limit={limit} />
              ))}
            </div>
          </section>

          <aside className="space-y-4">
            <section className="rounded-sm border bg-background">
              <SectionHeader
                title="License Request"
                description="Send this request to the license issuer when creating a client license."
                action={
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={!requestJson}
                    onClick={() => copyRequest(requestJson)}
                  >
                    {copiedRequest ? <CheckCheck className="size-4 text-green-500" /> : <Clipboard className="size-4" />}
                    Copy
                  </Button>
                }
              />
              <pre className="max-h-64 overflow-auto border-t bg-muted/30 p-3 text-xs leading-relaxed">
                {requestJson || 'License request is not available.'}
              </pre>
            </section>

            <section className="rounded-sm border bg-background">
              <SectionHeader title="Install License" description="Paste the compact .citadel-license value." />
              <div className="space-y-3 border-t p-4">
                <Textarea
                  value={licenseInput}
                  onChange={(event) => setLicenseInput(event.target.value)}
                  placeholder="eyJhbGciOiJFZDI1NTE5IiwidHlwIjoiY2l0YWRlbC1saWNlbnNlK2p3cyJ9..."
                  className="min-h-32 resize-y font-mono text-xs"
                />
                <div className="flex flex-wrap justify-end gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    disabled={installLicense.isPending || !licenseInput.trim()}
                    onClick={handleInstall}
                  >
                    {installLicense.isPending ? <Loader2 className="size-4 animate-spin" /> : <RotateCcw className="size-4" />}
                    Install
                  </Button>
                  <Button
                    type="button"
                    variant="destructive"
                    disabled={removeLicense.isPending || !license?.licenseId}
                    onClick={() => setConfirmRemoveOpen(true)}
                  >
                    {removeLicense.isPending ? <Loader2 className="size-4 animate-spin" /> : <Trash2 className="size-4" />}
                    Remove
                  </Button>
                </div>
              </div>
            </section>
          </aside>
        </div>
      </div>
      <ConfirmDeleteDialog
        type="License"
        open={confirmRemoveOpen}
        title="Remove License"
        confirmLabel="Remove"
        count={1}
        isPending={removeLicense.isPending}
        onOpenChange={setConfirmRemoveOpen}
        onConfirm={handleRemove}
        description={
          <>
            Removing the installed license returns Citadel to Community limits. Existing resources are kept, but create and
            enable actions may be blocked if current usage is over those limits.
          </>
        }
      />
    </div>
  );
}

function SectionHeader({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 p-4 sm:flex-row sm:items-start sm:justify-between">
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-foreground">{title}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
      {action}
    </div>
  );
}

function LicenseStatusBadge({ status }: { status: LicenseStatus }) {
  const active = ACTIVE_STATUSES.has(status);
  return (
    <Badge
      variant={active ? 'secondary' : 'destructive'}
      className={cn('rounded-sm', active && status === LicenseStatus.Valid && 'bg-green-500/10 text-green-700 dark:text-green-300')}
    >
      {active && status === LicenseStatus.Valid ? <ShieldCheck className="size-3.5" /> : null}
      {LICENSE_STATUS_LABELS[status] ?? status}
    </Badge>
  );
}

function QuotaRow({ limit }: { limit: LicenseLimitView }) {
  const current = Number(limit.current);
  const maximum = Number(limit.maximum);
  const percent = maximum > 0 ? Math.min(100, Math.round((current / maximum) * 100)) : 0;

  return (
    <div className="grid gap-3 p-4 sm:grid-cols-[220px_minmax(0,1fr)_120px] sm:items-center">
      <div className="min-w-0">
        <div className="truncate text-sm font-medium text-foreground">{LICENSE_LIMIT_LABELS[limit.limit] ?? limit.limit}</div>
        <div className="text-xs text-muted-foreground">{limit.overQuota ? 'Over licensed quota' : 'Within quota'}</div>
      </div>
      <div className="h-2 overflow-hidden rounded-sm bg-muted">
        <div className={cn('h-full bg-primary', limit.overQuota && 'bg-destructive')} style={{ width: `${percent}%` }} />
      </div>
      <div className="text-sm font-medium tabular-nums text-foreground sm:text-right">
        {current} / {maximum}
      </div>
    </div>
  );
}

function HeaderFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 border-l pl-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate font-medium text-foreground" title={value}>
        {value}
      </div>
    </div>
  );
}

function shortId(value: string) {
  return value ? value.slice(0, 8) : '-';
}

function shortFingerprint(value: string) {
  return value.length > 12 ? `${value.slice(0, 12)}...` : value;
}
