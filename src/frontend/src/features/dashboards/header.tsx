import { useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import { ChevronDown, Pencil } from 'lucide-react';
import { PageHeader } from '@/components/custom/page-header';
import { SearchField } from '@/components/custom/search-field';
import { Button } from '@/components/ui/button';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';

function Picker({ name }: { name: string }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const boards = useRead(
    'listDashboards',
    { query: { search, limit: 12, offset: 0 } },
    { enabled: open, staleTime: 0 },
  );
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button className="inline-flex max-w-full items-center gap-2 rounded-sm text-left hover:text-primary focus-visible:ring-2">
          <span className="truncate">{name}</span>
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 space-y-2 p-2">
        <SearchField placeholder="Find a dashboard…" onSearch={setSearch} />
        <div className="max-h-64 overflow-auto">
          {boards.isError ? (
            <p>Dashboards unavailable.</p>
          ) : boards.isFetching ? (
            <p>Loading…</p>
          ) : boards.data?.data.dashboards.length ? (
            boards.data.data.dashboards.map((b) => (
              <Link
                className="block truncate rounded px-2 py-1.5 text-sm hover:bg-accent"
                key={b.id}
                to={'/dashboards/' + b.id}
                onClick={() => setOpen(false)}>
                {b.name}
              </Link>
            ))
          ) : (
            <p className="p-2 text-sm text-muted-foreground">No matching dashboards.</p>
          )}
        </div>
        <Button asChild variant="ghost" className="w-full justify-start border-t">
          <Link to="/dashboards">All dashboards</Link>
        </Button>
      </PopoverContent>
    </Popover>
  );
}
export function DashboardHeader({
  name,
  description,
  edit,
  children,
}: {
  name: string;
  description: ReactNode;
  edit?: () => void;
  children: ReactNode;
}) {
  return (
    <PageHeader
      title={<Picker name={name} />}
      icon={<CitadelIcons.Dashboard className="size-5" />}
      description={description}
      actions={
        <>
          {edit && (
            <Button onClick={edit}>
              <Pencil />
              Edit dashboard
            </Button>
          )}
          {children}
        </>
      }
    />
  );
}
