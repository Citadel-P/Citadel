import type { ComponentType, ReactNode } from 'react';
import { ChevronDown, Tags } from 'lucide-react';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { truncate } from '@/lib/truncate';
import { CopyToClipboard } from './copy-to-clipboard';

export function DetailSection({
  title,
  description,
  icon: Icon,
  children,
  collapse,
}: {
  title: string;
  description?: string;
  icon: ComponentType<{ className?: string }>;
  children: ReactNode;
  collapse?: { open: boolean; onOpenChange: (open: boolean) => void };
}) {
  if (collapse) {
    return (
      <Collapsible asChild open={collapse.open} onOpenChange={collapse.onOpenChange}>
        <section className="min-w-0 overflow-hidden rounded-lg border bg-card shadow-xs">
          <h2>
            <CollapsibleTrigger className="group flex w-full items-start gap-3 bg-muted/15 px-(--surface-padding) py-3 text-left hover:bg-muted/30 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring">
              <span aria-hidden="true" className="mt-0.5 text-muted-foreground">
                <Icon className="size-4" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-semibold">{title}</span>
                {description && (
                  <span className="mt-1 block text-xs leading-relaxed text-muted-foreground">{description}</span>
                )}
              </span>
              <ChevronDown
                aria-hidden="true"
                className="mt-0.5 size-4 shrink-0 text-muted-foreground transition-transform group-data-[state=closed]:-rotate-90 motion-reduce:transition-none"
              />
            </CollapsibleTrigger>
          </h2>
          <CollapsibleContent className="border-t">
            <div className="min-w-0 space-y-4 p-(--surface-padding)">{children}</div>
          </CollapsibleContent>
        </section>
      </Collapsible>
    );
  }
  return (
    <section className="min-w-0 overflow-hidden rounded-lg border bg-card shadow-xs">
      <div className="flex items-start gap-3 border-b bg-muted/15 px-(--surface-padding) py-3">
        <span aria-hidden="true" className="mt-0.5 text-muted-foreground">
          <Icon className="size-4" />
        </span>
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">{title}</h2>
          {description && <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{description}</p>}
        </div>
      </div>
      <div className="min-w-0 space-y-4 p-(--surface-padding)">{children}</div>
    </section>
  );
}

export function DetailFacts({
  items,
  resource,
}: {
  items: { label: string; value: ReactNode }[];
  resource?: { id: string; name: string };
}) {
  const facts = [...items];
  if (resource?.id && resource.id !== resource.name) {
    facts.push({
      label: 'ID',
      value: (
        <CopyToClipboard
          textToCopy={resource.id}
          transform={(id) => truncate(id, 16)}
          textClassName="font-mono text-xs"
          groupClassName="[&_button]:visible [&_button]:opacity-0 hover:[&_button]:opacity-100 focus-within:[&_button]:opacity-100 [&_svg]:text-muted-foreground"
        />
      ),
    });
  }

  return (
    <dl className="grid grid-cols-1 gap-x-6 gap-y-5 sm:grid-cols-2 xl:grid-cols-3">
      {facts.map(({ label, value }) => (
        <div key={label} className="min-w-0">
          <dt className="mb-1.5 text-xs font-medium text-muted-foreground">{label}</dt>
          <dd className="min-w-0 text-sm font-medium leading-relaxed [overflow-wrap:anywhere]">
            {value ?? 'Not available'}
          </dd>
        </div>
      ))}
    </dl>
  );
}

export function DetailMetadata({ items, title = 'Labels' }: { items?: Record<string, string> | null; title?: string }) {
  const entries = Object.entries(items ?? {});
  if (!entries.length) return null;
  return (
    <DetailSection title={title} icon={Tags}>
      <dl className="divide-y">
        {entries.map(([key, value]) => (
          <div
            key={key}
            className="grid min-w-0 gap-1 py-3 first:pt-0 last:pb-0 sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)] sm:gap-6">
            <dt className="text-xs font-medium text-muted-foreground [overflow-wrap:anywhere]">{key}</dt>
            <dd className="min-w-0 font-mono text-xs leading-relaxed [overflow-wrap:anywhere]">{value || '—'}</dd>
          </div>
        ))}
      </dl>
    </DetailSection>
  );
}
