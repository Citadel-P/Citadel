import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { useSelectedResources } from '@/lib/atoms';
import { useDeleteDialog } from '@/lib/hooks';

export const ActionBar = ({ items }: { items: DockerVolumeResult[] }) => {
  const type = 'Volume';
  const { openDialog } = useDeleteDialog<DockerVolumeResult>({ type });
  const [selectedRows, _] = useSelectedResources<DockerVolumeResult>(type);

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedItems={selectedRows}
      allItems={items}
      resource={type}
      actionButtons={<ActionBarButtons selectedRows={selectedRows} openDialog={openDialog} />}
    />
  );
};
