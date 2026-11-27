import { Link } from 'react-router';
import { AppContent } from '@/components/custom/app-content';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Surface } from '@/components/custom/surface';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import { CitadelIcons } from '@/lib/icons';
import { SavedDashboard } from './board';

export default function DashboardHome() {
  const home = useRead('getDashboardDefault', undefined, { staleTime: 0, refetchOnMount: 'always' });
  if (home.isError)
    return <ResourceReadError page error={home.error} refetch={home.refetch} isFetching={home.isFetching} />;
  if (!home.isFetchedAfterMount || !home.data) return <Loader />;
  const id = home.data.data.dashboardId;
  if (id) return <SavedDashboard key={id} id={id} home />;
  return (
    <AppContent>
      <Surface className="flex flex-col items-center gap-3 py-12 text-center">
        <CitadelIcons.Dashboard aria-hidden="true" className="size-6 text-muted-foreground" />
        <h1 className="text-lg font-semibold">Choose a home dashboard</h1>
        <p className="text-sm text-muted-foreground">Open a dashboard and select Set as home.</p>
        <Button asChild variant="outline">
          <Link to="/dashboards">Browse dashboards</Link>
        </Button>
      </Surface>
    </AppContent>
  );
}
