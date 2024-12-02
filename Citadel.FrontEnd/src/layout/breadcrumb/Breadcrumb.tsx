import { useAppContext } from '@/AppProvider';
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
import { useNavigate } from 'react-router-dom';
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
  const { currentPlatform, currentContainer, route, isBreadcrumbHidden } = useAppContext();
  const navigate = useNavigate();
  if (isBreadcrumbHidden) return <></>;

  const crumbs: ICrumbs[] = [];
  if (route.path === paths[0]) {
    // Platforms
    crumbs.push({ title: 'Patforms', isActive: true });
  } else if (route.path === paths[1]) {
    // add docker platform
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: 'Add Platform', isActive: true });
  } else if (route.path === paths[2]) {
    // containers
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentPlatform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', isActive: true });
  } else if (route.path === paths[3]) {
    // container logs
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentContainer?.platform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', link: '/platforms/' + currentPlatform?.id + '/containers' });
    crumbs.push({ title: currentContainer?.name?.slice(1) ?? '', isActive: true, badge: { title: 'Logs' } });
  } else if (route.path === paths[4]) {
    // container logs
    crumbs.push({ title: 'Patforms', link: '/' });
    crumbs.push({ title: currentContainer?.platform?.name ?? '', link: '/platforms/' + currentPlatform?.id });
    crumbs.push({ title: 'Containers', link: '/platforms/' + currentPlatform?.id + '/containers' });
    crumbs.push({ title: currentContainer?.name?.slice(1) ?? '', isActive: true, badge: { title: 'Stats' } });
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
