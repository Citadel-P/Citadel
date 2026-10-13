import { ArrowLeft } from 'lucide-react';
import { Link, useSearchParams } from 'react-router';
import { backupReturnPath } from '../navigation';

export function BackToResource() {
  const [search] = useSearchParams();
  const returnTo = backupReturnPath(search);
  if (!returnTo) return null;

  const resourceLabel = returnTo.startsWith('/deployments/')
    ? 'Deployment'
    : returnTo.startsWith('/stacks/')
      ? 'Stack'
      : returnTo.startsWith('/swarm-services/')
        ? 'Swarm Service'
        : 'Container';

  return (
    <Link
      to={returnTo}
      className="group inline-flex w-fit items-center gap-2 rounded-sm py-1 text-sm font-medium text-muted-foreground transition-colors duration-200 hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring motion-reduce:transition-none">
      <ArrowLeft
        aria-hidden="true"
        className="size-4 shrink-0 transition-transform duration-200 ease-out group-hover:-translate-x-1 group-focus-visible:-translate-x-1 motion-reduce:transform-none motion-reduce:transition-none"
      />
      Back to {resourceLabel}
    </Link>
  );
}
