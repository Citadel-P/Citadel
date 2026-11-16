import { Activity, CircleHelp, Clock3, Layers, TriangleAlert } from 'lucide-react';
import type { JobStatusView } from '@/api/generated/api.types';
import { Card } from '@/components/ui/card';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import { displayState } from './presentation';
import { attention } from './descriptions';

export function JobsSummary({ jobs }: { jobs: JobStatusView[] }) {
  const states = jobs.map(displayState);
  const count = (state: string) => states.filter((value) => value === state).length;
  const items = [
    {
      label: 'Catching up',
      value: jobs.filter((j) => attention(j) === 'catchingUp').length,
      detail: 'Reported backlog and progress',
      Icon: Layers,
    },
    { label: 'Running', value: count('running'), detail: 'Executing an iteration', Icon: Activity, state: 'running' },
    {
      label: 'Retry scheduled',
      value: jobs.filter((j) => attention(j) === 'retrying').length,
      detail: 'Next check reported after failure',
      Icon: Clock3,
      state: 'backingOff',
    },
    {
      label: 'Needs attention',
      value: jobs.filter((j) => attention(j) === 'attention').length,
      detail: 'Failure without a reported next check',
      Icon: TriangleAlert,
      state: count('failed') > 0 ? 'failed' : 'backingOff',
    },
    {
      label: 'Status unavailable',
      value: count('unavailable'),
      detail: 'Iteration state unmeasured',
      Icon: CircleHelp,
      state: 'unavailable',
    },
  ];
  return (
    <dl aria-label="Job summary" className="grid grid-cols-2 gap-3 lg:grid-cols-5">
      {items.map(({ label, value, detail, Icon, state }) => {
        const tone = state && value > 0 ? getStateIconAppearance(state, 'job').className : 'text-muted-foreground';
        return (
          <Card key={label} className="gap-2 p-3 last:col-span-2 lg:last:col-span-1">
            <dt className="flex items-center justify-between gap-2 text-xs font-medium text-muted-foreground">
              {label}
              <Icon aria-hidden="true" className={`size-4 shrink-0 ${tone}`} />
            </dt>
            <dd className="space-y-1">
              <p className={`text-2xl font-semibold tabular-nums ${state && value > 0 ? tone : 'text-foreground'}`}>
                {value}
              </p>
              <p className="text-xs text-muted-foreground">{detail}</p>
            </dd>
          </Card>
        );
      })}
    </dl>
  );
}
