import { LicenseLimitView, LicenseStatus } from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { LICENSE_LIMIT_LABELS } from '@/features/license/license-labels';
import { useLocalStorage, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { AlertTriangle, X } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

const LICENSE_REMINDER_REFRESH_INTERVAL = 60 * 60 * 1000;

export function LicenseReminder() {
  const formatDate = useProfileDateTimeFormatter();
  const [dismissedKey, setDismissedKey] = useLocalStorage<string | null>('license-reminder-dismissed-key', null);
  const licenseQuery = useRead('getLicense', undefined, {
    retry: false,
    staleTime: LICENSE_REMINDER_REFRESH_INTERVAL,
    refetchInterval: LICENSE_REMINDER_REFRESH_INTERVAL,
    meta: { suppressErrorToast: true },
  });

  const license = licenseQuery.data?.data;
  const overQuota = useMemo(() => license?.limits?.filter((limit) => limit.overQuota) ?? [], [license?.limits]);

  const notice = useMemo(() => {
    if (!license) return null;

    if (license.status === LicenseStatus.GracePeriod) {
      return {
        tone: 'warning' as const,
        title: 'License in grace period',
        message: license.graceUntil
          ? `Renew before ${formatDate(license.graceUntil)} to avoid create and enable restrictions.`
          : 'Renew the license to avoid create and enable restrictions.',
      };
    }

    if (license.status === LicenseStatus.Expired) {
      return {
        tone: 'danger' as const,
        title: 'License expired',
        message: 'Renew your license or reduce usage to Community limits. Quota-increasing actions may be blocked.',
      };
    }

    if (overQuota.length > 0) {
      return {
        tone: 'warning' as const,
        title: 'License usage over limit',
        message: 'Renew your license or reduce usage to stay within the active limits.',
      };
    }

    return null;
  }, [formatDate, license, overQuota.length]);

  const overQuotaSummary = useMemo(() => formatOverQuota(overQuota), [overQuota]);
  const noticeKey =
    notice && license
      ? `${license.status}:${license.expiresAt ?? ''}:${license.graceUntil ?? ''}:${overQuotaSummary}`
      : null;

  if (!notice || !noticeKey || dismissedKey === noticeKey) return null;

  return (
    <div className="mx-auto px-4 pt-4 sm:px-6 lg:container">
      <Alert
        className={
          notice.tone === 'danger'
            ? 'rounded-sm border-destructive/40 bg-destructive/5'
            : 'rounded-sm border-amber-500/40 bg-amber-500/5'
        }>
        <AlertTriangle
          className={notice.tone === 'danger' ? 'text-destructive' : 'text-amber-600 dark:text-amber-300'}
        />
        <div className="flex min-w-0 flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <AlertTitle>{notice.title}</AlertTitle>
            <AlertDescription className="text-foreground/80">
              {notice.message}
              {overQuotaSummary ? (
                <span className="ml-1 font-medium text-foreground">Over limit: {overQuotaSummary}.</span>
              ) : null}
            </AlertDescription>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <Button asChild variant="outline" size="sm">
              <Link to="/license">View License</Link>
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-8"
              aria-label="Dismiss license reminder"
              onClick={() => setDismissedKey(noticeKey)}>
              <X className="size-4" />
            </Button>
          </div>
        </div>
      </Alert>
    </div>
  );
}

function formatOverQuota(limits: LicenseLimitView[]) {
  return limits
    .map((limit) => `${LICENSE_LIMIT_LABELS[limit.limit] ?? limit.limit} ${limit.current} / ${limit.maximum}`)
    .join(', ');
}
