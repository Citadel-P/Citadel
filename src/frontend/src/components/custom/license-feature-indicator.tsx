import { Lock } from 'lucide-react';

import { cn } from '@/lib/utils';

export type LicenseEdition = 'Team' | 'Enterprise';

export function LicenseFeatureIndicator({ edition, className }: { edition: LicenseEdition; className?: string }) {
  const requirement = `Requires a ${edition} license`;

  return (
    <span
      aria-label={requirement}
      title={requirement}
      className={cn(
        'inline-flex h-5 shrink-0 items-center gap-1 rounded-sm border border-amber-500/30 bg-amber-500/10 px-1.5',
        'text-[10px] font-medium leading-none text-amber-700 dark:text-amber-300',
        className,
      )}>
      <Lock className="size-3" aria-hidden="true" />
      <span>{edition}</span>
    </span>
  );
}

export function LicensedFeatureLabel({
  children,
  requiredLicense,
  className,
}: {
  children: React.ReactNode;
  requiredLicense?: LicenseEdition;
  className?: string;
}) {
  return (
    <span className={cn('flex w-full min-w-0 flex-1 items-center gap-2', className)}>
      <span className="min-w-0 truncate">{children}</span>
      {requiredLicense && <LicenseFeatureIndicator edition={requiredLicense} className="ml-auto" />}
    </span>
  );
}

export function LicensedFeatureDescription({
  children,
  requiredLicense,
  className,
  descriptionClassName,
  indicatorClassName,
}: {
  children: React.ReactNode;
  requiredLicense: LicenseEdition;
  className?: string;
  descriptionClassName?: string;
  indicatorClassName?: string;
}) {
  return (
    <div className={cn('flex w-full min-w-0 items-start justify-between gap-3', className)}>
      <div className={cn('min-w-0 text-sm leading-6 text-muted-foreground', descriptionClassName)}>{children}</div>
      <LicenseFeatureIndicator edition={requiredLicense} className={cn('mt-0.5 ml-auto', indicatorClassName)} />
    </div>
  );
}
