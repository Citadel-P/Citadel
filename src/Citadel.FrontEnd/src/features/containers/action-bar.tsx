import { useContainersContext } from './ContainersContext';
import { ContainerActionBarButtons } from './action-bar-buttons';
import { ActionBar } from '@/components/custom/action-bar';

export const ContainerActionBar = () => {
  const { containers, selectedRows, setDialogData } = useContainersContext();

  return (
    <ActionBar
      selectedRows={selectedRows}
      allItems={containers}
      resource="Container"
      actionButtons={<ContainerActionBarButtons selectedContainers={selectedRows} setDialogData={setDialogData} />}
    />
  );
};
