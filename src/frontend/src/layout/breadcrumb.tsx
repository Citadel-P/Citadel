import { Fragment, useEffect, useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { useAppContext } from '@/lib/context/app-context';
import { useSegmentTitle } from '@/lib/atoms';
import { truncate } from '@/lib/truncate';
import { capitalize } from '@/lib/utils';
import { ReversePluralResourceMap } from '@/api/types';

const titleizeSegment = (segment: string) =>
  segment
    .split('-')
    .map((s) => capitalize(s))
    .join(' ');

const singularizeSegment = (segment: string) => {
  const title = titleizeSegment(segment);
  return title.endsWith('ies') ? `${title.slice(0, -3)}y` : title.endsWith('s') ? title.slice(0, -1) : title;
};

export function useBreadcrumbItems() {
  const { pathname } = useLocation();
  const params = useParams();
  const { currentPlatform } = useAppContext();
  const [segmentTitle, setSegmentTitle] = useSegmentTitle();

  const segments = useMemo(() => {
    const pathSegments = pathname.split('/').filter(Boolean);
    return pathSegments.length === 0 ? ['platforms'] : pathSegments;
  }, [pathname]);

  const { platformId, type, resourceId, id } = params;

  useEffect(() => {
    setSegmentTitle(null);
  }, [resourceId, id, setSegmentTitle]);

  return useMemo(() => {
    const result: { title: string; link?: string }[] = [];
    let pathAcc = '';

    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i];
      pathAcc += `/${segment}`;

      if (i > 0 && segments[i - 1] == 'edit') {
        continue;
      }

      // Platform ID -> use platform name
      if (segment === platformId) {
        result.push({
          title: currentPlatform?.name ?? 'Platform',
          link: `/platforms/edit/${platformId}`,
        });
        continue;
      }

      // Resource ID -> use resource name
      if (segment === resourceId || segment === id) {
        const name = segmentTitle?.name.startsWith('/') ? segmentTitle?.name.slice(1) : (segmentTitle?.name ?? '');
        result.push({ title: truncate(name), link: pathAcc });
        continue;
      }

      // Static “add” -> “Add ResourceName”
      // /platforms/:id/:type/add
      // /:type/add or /:type/edit
      if (segment === 'add' || segment === 'edit') {
        const previousSegment = segments[i - 1];
        const typeStr =
          previousSegment && previousSegment !== type ? previousSegment : (type ?? previousSegment ?? 'Resource');
        const formattedType = typeStr
          .split('-')
          .map((s) => capitalize(s))
          .join('');
        const resourceKey = formattedType as keyof typeof ReversePluralResourceMap;
        let mapped = ReversePluralResourceMap[resourceKey] ?? formattedType;
        mapped = mapped.replace(/([A-Z])/g, ' $1').trim() as any;
        if (previousSegment && previousSegment !== type) {
          mapped = singularizeSegment(previousSegment) as any;
        }
        result.push({ title: `${capitalize(segment)} ${mapped}`, link: pathAcc });
        continue;
      }

      // Known static collections
      if (segment === 'platforms') {
        result.push({ title: 'Platforms', link: '/platforms' });
        continue;
      }

      // Resource type (containers, networks, images, volumes…)
      if (segment === type) {
        result.push({ title: titleizeSegment(segment), link: pathAcc });
        continue;
      }

      // Fallback
      result.push({ title: titleizeSegment(segment), link: pathAcc });
    }

    // Last breadcrumb is active -> no link
    if (result.length > 0) result[result.length - 1].link = undefined;

    return result;
  }, [segments, platformId, type, resourceId, id, currentPlatform, segmentTitle]);
}

export function BreadcrumbTrail({ className, compact = false }: { className?: string; compact?: boolean }) {
  const navigate = useNavigate();
  const crumbs = useBreadcrumbItems();
  const textClassName = compact
    ? 'hover:text-primary text-xs cursor-pointer max-w-40 truncate sm:max-w-64'
    : 'hover:text-primary text-xs cursor-pointer';
  const pageClassName = compact
    ? 'text-muted-foreground text-xs max-w-40 truncate sm:max-w-64'
    : 'text-muted-foreground text-xs';

  return (
    <Breadcrumb className={className}>
      <BreadcrumbList className={compact ? 'flex-nowrap overflow-hidden' : undefined}>
        {crumbs.map((crumb, i) => (
          <Fragment key={i}>
            <BreadcrumbItem className={compact ? 'min-w-0' : undefined}>
              {crumb.link ? (
                <BreadcrumbLink className={textClassName} onClick={() => navigate(crumb.link ?? '/')}>
                  {crumb.title}
                </BreadcrumbLink>
              ) : (
                <BreadcrumbPage className={pageClassName}>{crumb.title}</BreadcrumbPage>
              )}
            </BreadcrumbItem>

            {i < crumbs.length - 1 && <BreadcrumbSeparator className="shrink-0 text-[1px]" />}
          </Fragment>
        ))}
      </BreadcrumbList>
    </Breadcrumb>
  );
}

export default function BreadCrumb({ isSticky }: { isSticky: boolean }) {
  return (
    <div className={`sticky top-0 z-40 mx-auto px-4 lg:container sm:px-6 ${isSticky ? 'pt-0' : 'pt-3'}`}>
      <div className={`w-full border-border bg-background p-4 ${isSticky ? 'shadow-sm rounded-b-none' : 'rounded-lg'}`}>
        <BreadcrumbTrail />
      </div>
    </div>
  );
}
