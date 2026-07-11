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

export default function BreadCrumb({ isSticky }: { isSticky: boolean }) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const params = useParams();

  const { currentPlatform } = useAppContext();
  const [segmentTitle, setSegmentTitle] = useSegmentTitle();

  // Split path segments
  let segments = pathname.split('/').filter(Boolean);

  // Homepage "/"
  if (segments.length === 0) {
    segments = ['platforms'];
  }

  const { platformId, type, resourceId, id } = params;

  useEffect(() => {
    setSegmentTitle(null);
  }, [resourceId, id, setSegmentTitle]);

  const crumbs = useMemo(() => {
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
        const typeStr = previousSegment && previousSegment !== type ? previousSegment : (type ?? previousSegment ?? 'Resource');
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

  return (
    <div className={`sticky top-0 z-40 mx-auto px-4 lg:container sm:px-6 ${isSticky ? 'pt-0' : 'pt-3'}`}>
      <div className={`w-full border-border bg-background p-4 ${isSticky ? 'shadow-sm rounded-b-none' : 'rounded-lg'}`}>
        <Breadcrumb>
          <BreadcrumbList>
            {crumbs.map((crumb, i) => (
              <Fragment key={i}>
                <BreadcrumbItem>
                  {crumb.link ? (
                    <BreadcrumbLink
                      className="hover:text-primary text-xs cursor-pointer"
                      onClick={() => navigate(crumb.link ?? '/')}>
                      {crumb.title}
                    </BreadcrumbLink>
                  ) : (
                    <BreadcrumbPage className="text-muted-foreground text-xs">{crumb.title}</BreadcrumbPage>
                  )}
                </BreadcrumbItem>

                {i < crumbs.length - 1 && <BreadcrumbSeparator className="text-[1px]" />}
              </Fragment>
            ))}
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </div>
  );
}
