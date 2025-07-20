import { useAppContext } from '@/AppProvider';
import { AppPaths } from '@/AppRoutes';
import { Badge } from '@/components/ui/badge';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { useNavigate } from 'react-router';
import { Fragment, useMemo } from 'react';

interface ICrumbs {
  title: string;
  link?: string;
  isActive?: boolean;
  badge?: { title?: string } | undefined;
}

const BreadCrumb = ({ isSticky }: { isSticky: boolean }) => {
  const { currentPlatform, currentContainer, route } = useAppContext();

  const navigate = useNavigate();
  const generateCrumbs = useMemo((): ICrumbs[] => {
    const crumbs: ICrumbs[] = [];

    const routeMap: Record<string, () => void> = {
      [AppPaths.main]: () => crumbs.push({ title: 'Platforms', isActive: true }),
      [AppPaths.addPlatform]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: 'Add Platform', isActive: true });
      },
      [AppPaths.platformContainers]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Containers', isActive: true });
      },
      [AppPaths.containerLogs]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({
          title: currentContainer?.platformName ?? '',
          link: `/platforms/${currentContainer?.platformId}`,
        });
        crumbs.push({ title: 'Containers', link: `/platforms/${currentContainer?.platformId}/containers` });
        crumbs.push({
          title: currentContainer?.containerName?.slice(1) ?? '',
          isActive: true,
          badge: { title: 'Logs' },
        });
      },
      [AppPaths.containerStats]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({
          title: currentContainer?.platformName ?? '',
          link: `/platforms/${currentContainer?.platformId}`,
        });
        crumbs.push({ title: 'Containers', link: `/platforms/${currentContainer?.platformId}/containers` });
        crumbs.push({
          title: currentContainer?.containerName?.slice(1) ?? '',
          isActive: true,
          badge: { title: 'Stats' },
        });
      },
      [AppPaths.containerInspect]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentContainer?.platformName ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Containers', link: `/platforms/${currentPlatform?.id}/containers` });
        crumbs.push({
          title: currentContainer?.containerName?.slice(1) ?? '',
          isActive: true,
          badge: { title: 'Inspect' },
        });
      },
      [AppPaths.registries]: () => crumbs.push({ title: 'Registries', isActive: true }),
      [AppPaths.addRegistry]: () => {
        crumbs.push({ title: 'Registries', link: '/registries' });
        crumbs.push({ title: 'Add Registry', isActive: true });
      },
      [AppPaths.editRegistry]: () => {
        crumbs.push({ title: 'Registries', link: '/registries' });
        crumbs.push({ title: 'Edit Registry', isActive: true });
      },
      [AppPaths.images]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Images', badge: { title: 'Local' }, isActive: true });
      },
      [AppPaths.localImages]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Images', badge: { title: 'Local' }, isActive: true });
      },
      [AppPaths.externalImages]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Images', badge: { title: 'External' }, isActive: true });
      },
      [AppPaths.networks]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Networks', isActive: true });
      },
      [AppPaths.addNetwork]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Networks', link: `/platforms/${currentPlatform?.id}/networks` });
        crumbs.push({ title: 'Add Network', isActive: true });
      },
      [AppPaths.volumes]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Volumes', isActive: true });
      },
      [AppPaths.addVolume]: () => {
        crumbs.push({ title: 'Platforms', link: '/' });
        crumbs.push({ title: currentPlatform?.name ?? '', link: `/platforms/${currentPlatform?.id}` });
        crumbs.push({ title: 'Volumes', link: `/platforms/${currentPlatform?.id}/volumes` });
        crumbs.push({ title: 'Add Volume', isActive: true });
      },
    };

    routeMap[route?.path ?? '']?.();

    return crumbs;
  }, [route, currentPlatform, currentContainer]);

  return (
    <div className={`sticky top-0 z-40 mx-auto px-4 lg:container sm:px-6 ${isSticky ? 'pt-0' : 'pt-3'}`}>
      <div className={`w-full border-border bg-background p-4 ${isSticky ? 'shadow-md rounded-b-none' : 'rounded-lg'}`}>
        <Breadcrumb>
          <BreadcrumbList>
            {generateCrumbs.map((crumb, i) => (
              <Fragment key={i}>
                <BreadcrumbItem>
                  {!crumb.isActive ? (
                    <BreadcrumbLink
                      className="hover:text-primary text-sm cursor-pointer"
                      onClick={() => navigate(crumb.link ?? '/')}>
                      {crumb.title}
                    </BreadcrumbLink>
                  ) : (
                    <BreadcrumbPage className="text-muted-foreground">{crumb.title}</BreadcrumbPage>
                  )}
                </BreadcrumbItem>
                {crumb.badge && (
                  <Badge variant="secondary" className="px-1.5 font-normal">
                    {crumb.badge.title}
                  </Badge>
                )}
                {i < generateCrumbs.length - 1 && <BreadcrumbSeparator />}
              </Fragment>
            ))}
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </div>
  );
};

export default BreadCrumb;
