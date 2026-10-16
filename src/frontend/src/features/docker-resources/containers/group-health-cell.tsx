import { Activity } from 'lucide-react';
import { ContainerHealthStatus as Status } from '@/api/generated/api.types';
import { HealthCard } from '@/components/custom/health-card';
import { StateBadge } from '@/components/custom/state-badge';
import { healthLabels, healthTone } from './health';
import type { ContainerGroupHealth } from './group-health';

export function ContainerGroupHealthCell({ name, summary }: { name: string; summary?: ContainerGroupHealth }) {
  const status = summary?.status ?? Status.Unknown;
  const tone = healthTone(status === 'Partial' ? Status.Unknown : status);
  const label =
    status === 'Partial' ? 'Partial health' : status === Status.NoCheck ? 'No checks' : healthLabels[status];
  return (
    <HealthCard
      title="Stack health"
      resourceName={name}
      triggerLabel={`Stack health: ${label}`}
      triggerClassName={tone}
      trigger={<Activity className={`size-4 ${tone}`} aria-hidden="true" />}
      status={<StateBadge kind="health" value={status} label={label} indicator={<Activity aria-hidden="true" />} />}
      summary={
        <div className="space-y-3">
          <p className="text-sm font-medium break-words">{name}</p>
          {summary?.total ? (
            <>
              <dl className="space-y-2 text-sm">
                {Object.entries(summary.counts)
                  .filter(([, count]) => count > 0)
                  .map(([state, count]) => (
                    <div key={state} className="flex items-center justify-between gap-4">
                      <dt className="flex items-center gap-2">
                        <Activity className={`size-3.5 ${healthTone(state as Status)}`} aria-hidden="true" />
                        {healthLabels[state as Status]}
                      </dt>
                      <dd className="tabular-nums">{count}</dd>
                    </div>
                  ))}
              </dl>
              <p className="border-t pt-3 text-xs text-muted-foreground">
                Based on {summary.total} accessible {summary.total === 1 ? 'container' : 'containers'}, including those
                hidden by filters.
              </p>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">Container health is unavailable.</p>
          )}
        </div>
      }
    />
  );
}
