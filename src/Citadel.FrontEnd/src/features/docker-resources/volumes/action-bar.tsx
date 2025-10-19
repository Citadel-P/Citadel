import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { useSelectedResources } from '@/lib/atoms';
import { useDeleteDialog } from '@/lib/hooks';

export const ActionBar = ({ items }: { items: DockerVolumeResult[] }) => {
  const { openDialog } = useDeleteDialog<DockerVolumeResult>({ type : 'Volume' });
  const [selectedRows, _] = useSelectedResources<DockerVolumeResult>('Volume');

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={items}
      resource="Volume"
      actionButtons={<ActionBarButtons selectedRows={selectedRows} openDialog={openDialog} />}
    />
  );
};
