import { useMemo, useState } from 'react';
import { Plus } from 'lucide-react';
import { Surface } from '@/components/custom/surface';
import { Button } from '@/components/ui/button';
import { CitadelIcons } from '@/lib/icons';
import { ReadAdmissionContext, createReadAdmission } from '@/lib/read-admission';
import { OverviewTiles } from './overview';
import { supportedConfiguration } from './catalogue';
import { OrderedTiles } from './tile';
import type { DashboardWidget } from '@/api/generated/api.types';
import { dashboardRequirements } from './requirements';
import { UnsupportedConfiguration } from './unsupported-configuration';

export type SavedTile = DashboardWidget;
export function DashboardGrid({
  configuration,
  inactive = false,
  onAddWidget,
}: {
  configuration: unknown;
  inactive?: boolean;
  onAddWidget?: () => void;
}) {
  const [admit] = useState(() => createReadAdmission(2));
  const config = supportedConfiguration(configuration) ? configuration : undefined;
  const widgets = config?.widgets;
  const tiles = useMemo(
    () => widgets && [...widgets].sort((a, b) => a.layout.y - b.layout.y || a.layout.x - b.layout.x),
    [widgets],
  );
  const plan = useMemo(() => dashboardRequirements(tiles ?? []), [tiles]);
  if (inactive) return null;
  if (!config || !tiles) return <UnsupportedConfiguration />;
  if (!tiles.length) {
    return (
      <Surface className="flex min-h-60 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
        <span
          aria-hidden="true"
          className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
          <CitadelIcons.Dashboard className="size-5" />
        </span>
        <h2 className="text-base font-semibold">No widgets yet</h2>
        {onAddWidget && (
          <>
            <p className="text-sm text-muted-foreground">
              Add charts, recent activity, or resource status from the widget library.
            </p>
            <Button onClick={onAddWidget}>
              <Plus /> Add widget
            </Button>
          </>
        )}
      </Surface>
    );
  }
  return (
    <ReadAdmissionContext.Provider value={admit}>
      <OrderedTiles tiles={tiles}>
        <OverviewTiles plan={plan} tags={config.tags} range={config.rangeMinutes} />
      </OrderedTiles>
    </ReadAdmissionContext.Provider>
  );
}
