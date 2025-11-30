import { Fragment, useMemo } from 'react';
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

export default function BreadCrumb({ isSticky }: { isSticky: boolean }) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const params = useParams();

  const { currentPlatform, currentContainer } = useAppContext();
  const [segmentTitle, _] = useSegmentTitle();

  // Split path segments
  let segments = pathname.split('/').filter(Boolean);

  // Homepage "/"
  if (segments.length === 0) {
    segments = ['platforms'];
  }

  const crumbs = useMemo(() => {
    const result: { title: string; link?: string }[] = [];
    let pathAcc = '';

    const { platformId, type, resourceId, id } = params;

    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i];
      pathAcc += `/${segment}`;

      if (i > 0 && segments[i-1] == 'edit') {
        continue;
      }

      // CASE 1: Platform ID → use platform name
      if (segment === platformId) {
        result.push({
          title: currentPlatform?.name ?? 'Platform',
          link: pathAcc,
        });
        continue;
      }

      // CASE 2: Resource ID → use resource name
      if (segment === resourceId || segment === id) {
        const name = segmentTitle?.name.startsWith('/')
          ? segmentTitle?.name.slice(1)
          : (segmentTitle?.name ?? currentContainer?.name?.slice(1) ?? segment);

        result.push({ title: truncate(name), link: pathAcc });
        continue;
      }

      // CASE 3: Static “add” → “Add ResourceName”
      // /platforms/:id/:type/add
      // /:type/add or /:type/edit
      if (segment === 'add' || segment === 'edit') {
        const resourceName = capitalize(type ?? 'Resource');
        const resourceKey = resourceName as keyof typeof ReversePluralResourceMap;
        const mapped = ReversePluralResourceMap[resourceKey] ?? resourceName;
        result.push({ title: `${capitalize(segment)} ${mapped}`, link: pathAcc });
        continue;
      }

      // CASE 4: Known static collections
      if (segment === 'platforms') {
        result.push({ title: 'Platforms', link: '/platforms' });
        continue;
      }

      // ──────────────────────────
      // CASE 7: Resource type (containers, networks, images, volumes…)
      // ──────────────────────────
      if (segment === type) {
        result.push({ title: capitalize(type), link: pathAcc });
        continue;
      }

      // ──────────────────────────
      // Fallback
      // ──────────────────────────
      result.push({ title: segment, link: pathAcc });
    }

    // Last breadcrumb is active → no link
    if (result.length > 0) result[result.length - 1].link = undefined;

    return result;
  }, [segments, params, currentPlatform, currentContainer, segmentTitle]);

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
