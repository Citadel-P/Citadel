import { ChartArea } from 'lucide-react';
import { catalogue, type WidgetKind } from './catalogue';

export function WidgetThumbnail({ kind, color }: { kind: WidgetKind; color?: string }) {
  const { Icon } = catalogue[kind];
  const content = (() => {
    switch (kind) {
      case 'Metrics':
        return <ChartArea className="h-6 w-10" />;
      case 'FleetSummary':
        return (
          <div className="flex gap-1">
            {[0, 1, 2].map((i) => (
              <span key={i} className="h-3 w-2.5 border-y-2 border-muted-foreground/60" />
            ))}
          </div>
        );
      case 'ResourceState':
        return (
          <div className="flex h-2 w-9 overflow-hidden rounded-full">
            <span className="w-2/3 bg-success" />
            <span className="grow bg-muted-foreground" />
          </div>
        );
      case 'Notes':
        return (
          <div className="grid w-8 gap-1">
            <span className="h-0.5 bg-muted-foreground" />
            <span className="h-0.5 bg-muted-foreground/50" />
            <span className="h-0.5 w-1/2 bg-primary" />
          </div>
        );
      default:
        return (
          <div className="grid gap-0.5">
            {[0, 1, 2].map((i) => (
              <div key={i} className="flex items-center gap-1">
                <Icon
                  className={
                    kind === 'NeedsAttention' && i === 0 ? 'size-2 text-warning' : 'size-2 text-muted-foreground'
                  }
                />
                <span className="h-1 w-5 rounded-sm bg-muted-foreground/35" />
              </div>
            ))}
          </div>
        );
    }
  })();
  return (
    <span
      aria-hidden="true"
      style={{ color }}
      className="dashboard-widget-thumbnail flex h-10 w-14 shrink-0 items-center justify-center rounded-md border text-primary">
      {content}
    </span>
  );
}
