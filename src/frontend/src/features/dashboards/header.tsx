import { ResourceDetailHeader } from '@/components/custom/resource-detail-header';
import { useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import { Check, ChevronDown, Pencil } from 'lucide-react';
import { MarkdownDescription } from '@/components/custom/markdown-description';
import { SearchField } from '@/components/custom/search-field';
import { ButtonGroup } from '@/components/ui/button-group';
import { Button } from '@/components/ui/button';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { CitadelIcons } from '@/lib/icons';
import { audiences } from './visibility';
import { useRead } from '@/lib/hooks';

function Picker({ name, id }: { name: string; id: string }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const boards = useRead(
    'listDashboards',
    { query: { search, limit: 12, offset: 0 } },
    { enabled: open, staleTime: Infinity },
  );
  const saved = boards.isError ? [] : (boards.data?.data.dashboards ?? []);
  const searchTerm = search.trim().toLowerCase();
  const items: { id: string; name: string; audience?: string }[] = [];
  if (!searchTerm && !boards.isError && boards.data && !saved.some((board) => board.id === id)) {
    items.push({ id, name });
  }
  for (const board of saved) {
    items.push({ id: board.id, name: board.name, audience: audiences[board.visibility] });
  }
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setSearch('');
      }}>
      <PopoverTrigger asChild>
        <button className="inline-flex max-w-full items-center gap-2 rounded-sm text-left hover:text-primary focus-visible:ring-2">
          <span className="truncate">{name}</span>
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" sideOffset={8} className="flex w-72 max-w-[calc(100vw-2rem)] flex-col gap-2.5 p-2">
        <SearchField className="mb-0" placeholder="Find a dashboard…" onSearch={setSearch} />
        <div className="flex max-h-64 flex-col gap-1 overflow-auto">
          {items.map((board) => {
            const current = board.id === id;
            const path = '/dashboards/' + board.id;
            return (
              <Link
                aria-current={current ? 'page' : undefined}
                className="flex shrink-0 items-center gap-2 rounded-md px-2.5 py-1.5 text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-ring aria-[current=page]:bg-accent aria-[current=page]:font-medium aria-[current=page]:text-accent-foreground"
                key={path}
                to={path}
                onClick={(event) => {
                  if (current) event.preventDefault();
                  setOpen(false);
                  setSearch('');
                }}>
                <CitadelIcons.Dashboard aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1">
                  <span className="block truncate">{board.name}</span>
                  {board.audience && (
                    <>
                      {' '}
                      <span className="block text-xs font-normal text-muted-foreground">{board.audience}</span>
                    </>
                  )}
                </span>
                {current && <Check aria-hidden="true" className="size-4 text-primary" />}
              </Link>
            );
          })}
          {boards.isError ? (
            <p>Dashboards unavailable.</p>
          ) : !boards.data && boards.isFetching ? (
            <p>Loading…</p>
          ) : !items.length ? (
            <p className="p-2 text-sm text-muted-foreground">No matching dashboards.</p>
          ) : null}
        </div>
        <div className="border-t pt-1.5">
          <Button asChild variant="ghost" size="sm" className="w-full justify-start">
            <Link to="/dashboards">All dashboards</Link>
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
export function DashboardHeader({
  id,
  name,
  description,
  badge,
  secondary,
  edit,
  children,
}: {
  name: string;
  id: string;
  description?: string;
  badge: ReactNode;
  edit?: () => void;
  children?: ReactNode;
  secondary?: ReactNode;
}) {
  return (
    <ResourceDetailHeader
      icon={<CitadelIcons.Dashboard className="size-5" />}
      actions={
        <ButtonGroup aria-label="Dashboard actions" className="flex-wrap">
          {edit && (
            <Button variant="outline" onClick={edit}>
              <Pencil /> Edit dashboard
            </Button>
          )}
          {children}
        </ButtonGroup>
      }>
      <div className="flex flex-wrap items-center gap-2">
        <h1 className="min-w-0 text-2xl font-semibold tracking-tight">
          <Picker name={name} id={id} />
        </h1>
        {badge}
      </div>
      {description && (
        <div className="text-sm leading-relaxed text-muted-foreground [overflow-wrap:anywhere]">
          <MarkdownDescription>{description}</MarkdownDescription>
        </div>
      )}
      {secondary && <div className="mt-2 flex flex-wrap items-center gap-2">{secondary}</div>}
    </ResourceDetailHeader>
  );
}
