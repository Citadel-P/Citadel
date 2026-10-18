import { NetworksTable } from './table';
import { NetworkColumnsButton } from './table-columns';
import { NetworkFilters } from './filters';

export function NetworkViewControls() {
  return (
    <>
      <NetworkFilters />
      <NetworkColumnsButton />
    </>
  );
}

export function NetworkViews(props: React.ComponentProps<typeof NetworksTable>) {
  return <NetworksTable {...props} />;
}
