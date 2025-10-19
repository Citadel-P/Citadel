import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';
import { useDeleteDialog } from '@/lib/hooks';
import { useSelectedResources } from '@/lib/atoms';
import { DockerNetworkResult } from '@/api/generated/api.types';

export const ActionBar = ({ items }: { items: DockerNetworkResult[] }) => {
  const type = 'Network';
  const { openDialog } = useDeleteDialog<DockerNetworkResult>({ type });
  const [selectedRows, _] = useSelectedResources<DockerNetworkResult>(type);

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={items}
      resource={type}
      actionButtons={<ActionBarButtons selectedRows={selectedRows} openDialog={openDialog} />}
    />
  );
};
