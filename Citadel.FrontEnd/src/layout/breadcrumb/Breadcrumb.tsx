import { AppContext } from '@/AppProvider';
import { useContextSelector } from 'use-context-selector';
import { paths } from '@/AppRoutes';
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
import { Fragment } from 'react/jsx-runtime';

interface ICrumbs {
  title: string;
  link?: string;
  isActive?: boolean;
  badge?: ICrumbBadge | undefined;
}

interface ICrumbBadge {
  title?: string | undefined;
}

const BreadCrumb = () => {
  const route = useContextSelector(AppContext, (v) => v?.route);
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const currentContainer = useContextSelector(AppContext, (v) => v?.currentContainer);
  const isBreadcrumbHidden = useContextSelector(AppContext, (v) => v?.isBreadcrumbHidden);
  const navigate = useNavigate();
  if (isBreadcrumbHidden) return <></>;

  const crumbs: ICrumbs[] = [];
  if (route?.path === paths[2]) {
    // platforms
    crumbs.push({ title: 'Patforms', isActive: true });
  } else if (route?.path === paths[3]) {
    // add docker platform
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: 'Add Platform', isActive: true });
  } else if (route?.path === paths[4]) {
    // containers
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentPlatform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', isActive: true });
  } else if (route?.path === paths[5]) {
    // container logs
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentContainer?.platform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', link: '/platforms/' + currentPlatform?.id + '/containers' });
    crumbs.push({ title: currentContainer?.name?.slice(1) ?? '', isActive: true, badge: { title: 'Logs' } });
  } else if (route?.path === paths[6]) {
    // container stats
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentContainer?.platform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', link: '/platforms/' + currentPlatform?.id + '/containers' });
    crumbs.push({ title: currentContainer?.name?.slice(1) ?? '', isActive: true, badge: { title: 'Stats' } });
  } else if (route?.path === paths[7]) {
    // container stats
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentContainer?.platform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', link: '/platforms/' + currentPlatform?.id + '/containers' });
    crumbs.push({ title: currentContainer?.name?.slice(1) ?? '', isActive: true, badge: { title: 'Inspect' } });
  } else if (route?.path === paths[8]) {
    // registries
    crumbs.push({ title: 'Registries', isActive: true });
  } else if (route?.path === paths[9]) {
    // add docker platform
    crumbs.push({ title: 'Registries', link: '/registries' });
    crumbs.push({ title: 'Add registry', isActive: true });
  } else if (route?.path === paths[10]) {
    // edit docker platform
    crumbs.push({ title: 'Registries', link: '/registries' });
    crumbs.push({ title: 'Edit registry', isActive: true });
  } else if (route?.path === paths[11]) {
    // images
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentPlatform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Images', badge: { title: 'local' }, isActive: true });
  } else if (route?.path === paths[12]) {
    // images local
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentPlatform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Images', badge: { title: 'local' }, isActive: true });
  } else if (route?.path === paths[13]) {
    // images local
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentPlatform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Images', badge: { title: 'external' }, isActive: true });
  }

  return (
    <div className="mx-auto px-4 pt-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <Breadcrumb>
          <BreadcrumbList>
            {crumbs.map((crumb, i) =>
              !crumb.isActive ? (
                <Fragment key={i}>
                  <BreadcrumbItem>
                    <BreadcrumbLink
                      className="hover:text-primary text-sm cursor-pointer"
                      onClick={() => navigate(crumb.link ?? '/')}>
                      {crumb.title}
                    </BreadcrumbLink>
                  </BreadcrumbItem>
                  <BreadcrumbSeparator />
                </Fragment>
              ) : (
                <Fragment key={i}>
                  <BreadcrumbItem>
                    <BreadcrumbPage className="text-muted-foreground">{crumb.title}</BreadcrumbPage>
                  </BreadcrumbItem>
                  {crumb.badge !== undefined ? (
                    <Badge variant="secondary" className="px-1.5 font-normal">
                      {crumb.badge.title}
                    </Badge>
                  ) : (
                    <></>
                  )}
                </Fragment>
              ),
            )}
          </BreadcrumbList>
        </Breadcrumb>
      </div>
    </div>
  );
};

export default BreadCrumb;
