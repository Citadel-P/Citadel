import { LicenseStatus } from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { useLocalStorage } from '@/lib/hooks';
import { AlertTriangle, X } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

export function LicenseReminder() {
  const [dismissedKey, setDismissedKey] = useLocalStorage<string | null>('license-reminder-dismissed-key', null);
  const { entitlements: license } = useLicenseEntitlements();

  const notice = useMemo(() => {
    if (!license) return null;

    if (license.status === LicenseStatus.GracePeriod) {
      return {
        tone: 'warning' as const,
        title: 'License in grace period',
        message: 'Renew the license before the grace period ends to keep Team operations active.',
      };
    }

    if (license.status === LicenseStatus.Expired) {
      return {
        tone: 'danger' as const,
        title: 'License expired',
        message: 'Citadel is using Community capabilities. Paid configuration is preserved but remains paused.',
      };
    }

    return null;
  }, [license]);

  const noticeKey = notice && license ? `${license.status}:${license.effectiveEdition}` : null;

  if (!notice || !noticeKey || dismissedKey === noticeKey) return null;

  return (
    <div className="mx-auto w-full max-w-[1440px] px-4 pt-4 sm:px-6">
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
            <AlertDescription className="text-foreground/80">{notice.message}</AlertDescription>
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
