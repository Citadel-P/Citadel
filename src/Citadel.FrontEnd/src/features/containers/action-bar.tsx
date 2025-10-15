import { useContainersContext } from './ContainersContext';
import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';

export const ActionBar = () => {
  const { containers, selectedRows, setDialogData } = useContainersContext();

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={containers}
      resource="Container"
      actionButtons={<ActionBarButtons selectedContainers={selectedRows} setDialogData={setDialogData} />}
    />
  );
};
