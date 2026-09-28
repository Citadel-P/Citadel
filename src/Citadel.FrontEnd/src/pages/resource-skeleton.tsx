import { AppContent } from '@/components/custom/app-content';
import { Skeleton } from '@/components/ui/skeleton';
import { TableSkeleton } from '@/components/ui/table-skeleton';
import { cn } from '@/lib/utils';

export function ResourceOverviewSkeleton({ count = 4 }: { count?: number }) {
  return (
    <div aria-hidden="true" className="space-y-2">
      <Skeleton className="h-5 w-32" />
      <div
        className={cn(
          'grid grid-cols-2 gap-[calc(var(--section-gap)/2)]',
          count === 3 ? 'sm:grid-cols-3' : count === 5 ? 'xl:grid-cols-5' : 'xl:grid-cols-4',
        )}>
        {Array.from({ length: count }, (_, index) => (
          <div
            key={index}
            className="rounded-lg border bg-card px-[calc(var(--surface-padding)-0.25rem)] py-[calc(var(--surface-padding)-0.5rem)]">
            <Skeleton className="h-5 w-3/4" />
            <Skeleton className="mt-1 h-7 w-10" />
            <Skeleton className="mt-0.5 h-4 w-full" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function ResourceCardsSkeleton() {
  return (
    <div role="status" aria-label="Loading platforms" className="space-y-(--section-gap)">
      <span className="sr-only">Loading platforms…</span>
      {[0, 1].map((index) => (
        <div key={index} aria-hidden="true" className="space-y-5 rounded-lg border bg-card p-(--surface-padding)">
          <div className="flex items-center gap-3">
            <Skeleton className="size-10" />
            <Skeleton className="h-5 w-40" />
          </div>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            {[0, 1, 2, 3].map((item) => (
              <div key={item} className="space-y-2">
                <Skeleton className="h-3 w-2/3" />
                <Skeleton className="h-5 w-1/3" />
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

export function ResourceSkeleton({
  variant = 'list',
  embedded = false,
}: {
  variant?: 'list' | 'detail' | 'form' | 'platform';
  embedded?: boolean;
}) {
  const Wrapper = embedded ? 'div' : AppContent;
  const isList = variant === 'list' || variant === 'platform';
  return (
    <Wrapper role="status" aria-label="Loading resource" className="min-w-0">
      <span className="sr-only">Loading resource…</span>
      <div aria-hidden="true" className="flex min-w-0 flex-col gap-(--section-gap)">
        <div
          className={cn(
            'flex flex-wrap items-center justify-between gap-4',
            !isList && 'rounded-lg border bg-card p-(--surface-padding)',
          )}>
          <div className="flex min-w-0 items-center gap-3">
            <Skeleton className="size-11 shrink-0" />
            <div className="space-y-2">
              <Skeleton className="h-6 w-40 sm:w-52" />
              <Skeleton className="h-3 w-32" />
            </div>
          </div>
          <Skeleton className="h-9 w-24" />
        </div>
        {isList ? (
          <>
            <div className="flex flex-wrap justify-between gap-3 rounded-lg border bg-card p-3">
              <Skeleton className="h-9 w-full sm:w-72" />
              <Skeleton className="h-9 w-28" />
            </div>
            {variant === 'platform' ? <ResourceCardsSkeleton /> : <TableSkeleton />}
          </>
        ) : (
          <>
            {variant === 'detail' && <ResourceOverviewSkeleton />}
            <div className="flex gap-3 rounded-lg border bg-card p-2">
              {[0, 1, 2].map((tab) => (
                <Skeleton key={tab} className="h-8 w-20" />
              ))}
            </div>
            <div className="flex min-w-0 gap-(--section-gap)">
              {variant === 'form' && (
                <div className="hidden w-48 shrink-0 space-y-3 rounded-lg border bg-card p-3 lg:block">
                  {[0, 1, 2, 3, 4].map((item) => (
                    <Skeleton key={item} className="h-8 w-full" />
                  ))}
                </div>
              )}
              <div className="min-w-0 flex-1 space-y-6 rounded-lg border bg-card p-(--surface-padding)">
                <Skeleton className="h-5 w-32" />
                {[0, 1, 2].map((field) => (
                  <div key={field} className="space-y-2">
                    <Skeleton className="h-3 w-24" />
                    <Skeleton className="h-9 w-full max-w-xl" />
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
    </Wrapper>
  );
}
