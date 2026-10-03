/** Match a resource family without confusing similarly prefixed routes. */
export function isSidebarRouteActive(pathname: string, route?: string | null) {
  if (!route) return false;
  if (route === '/') return ['/', '/platforms', '/platforms/', '/platforms/add'].includes(pathname);
  return pathname === route || pathname.startsWith(`${route}/`);
}
