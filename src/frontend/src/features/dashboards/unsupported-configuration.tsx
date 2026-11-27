import { FileWarning } from 'lucide-react';
import { Link } from 'react-router';
import { ErrorPanel } from '@/components/custom/page-error';
import { Button } from '@/components/ui/button';

export function UnsupportedConfiguration() {
  return (
    <ErrorPanel
      title="Can’t display dashboard"
      message="This dashboard uses an unsupported configuration."
      illustration={
        <FileWarning aria-hidden="true" className="my-6 size-24 text-muted-foreground" strokeWidth={1.25} />
      }>
      <Button asChild>
        <Link to="/dashboards">View dashboards</Link>
      </Button>
    </ErrorPanel>
  );
}
