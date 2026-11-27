import { useState } from 'react';
import { useNavigate } from 'react-router';
import { Check, Eye, House, Trash } from 'lucide-react';
import type { DashboardSummaryView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { ActionButton } from '@/components/custom/action-with-dialog';
import { DropdownActionButton, RowActionDropdown } from '@/components/custom/dropdown-with-dialog';
import type { ButtonGroupComponent, DropdownActionComponent } from '@/pages/types';
import { PreferenceDialog, useDashboardHomeAction } from './home-preference';
import { DashboardDeleteDialog } from './delete-dialog';
import { SharingDialog, SharingItem, useCanManageSharing } from './sharing';

const { dropdown, group } = createActionsBuilder<DashboardSummaryView>()
  .addAction({
    key: 'open',
    title: 'Open',
    type: 'command',
    icon: Eye,
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const board = Array.isArray(resources) ? resources[0] : resources;
      return {
        canExecute: !Array.isArray(resources) || resources.length === 1,
        run: () => navigate('/dashboards/' + board.id),
      };
    },
  })
  .build();

const rowActions: Record<string, DropdownActionComponent<DashboardSummaryView>> = {
  open: dropdown.open,
  home: function Home({ resource, onAction }) {
    const home = useDashboardHomeAction(resource);
    return (
      <DropdownActionButton
        title={home.label}
        disabled={home.disabled}
        icon={home.current ? <Check /> : <House />}
        onClick={() => onAction?.('home')}
      />
    );
  },
  sharing: function Sharing({ resource, onAction }) {
    const canShare = useCanManageSharing(resource);
    return canShare ? <SharingItem board={resource} select={() => onAction?.('sharing')} /> : null;
  },
  delete: ({ resource, onAction }) =>
    resource.canDelete ? (
      <DropdownActionButton
        title="Delete"
        variant="destructive"
        separatorBefore
        icon={<Trash />}
        onClick={() => onAction?.('delete')}
      />
    ) : null,
};

export function DashboardRowActions({ board }: { board: DashboardSummaryView }) {
  const [action, setAction] = useState<{ key: string; board: DashboardSummaryView }>();
  const close = () => setAction(undefined);
  return (
    <>
      <RowActionDropdown
        resource={board}
        actions={rowActions}
        onActionSelect={({ key }) => setAction({ key, board })}
      />
      {action?.key === 'delete' && board.canDelete && <DashboardDeleteDialog boards={[action.board]} close={close} />}
      {action?.key === 'home' && <PreferenceDialog board={action.board} close={close} />}
      {action?.key === 'sharing' && <SharingDialog board={action.board} close={close} />}
    </>
  );
}

export function SetSelectedHome({ resources }: { resources: DashboardSummaryView[] }) {
  const [board, setBoard] = useState<DashboardSummaryView>();
  const home = useDashboardHomeAction(resources.length === 1 ? resources[0] : undefined);
  return (
    <>
      <ActionButton
        title={home.label}
        variant="outline"
        iconPosition="left"
        icon={home.current ? <Check className="size-4" /> : <House className="size-4" />}
        disabled={resources.length !== 1 || home.disabled}
        onClick={() => setBoard(resources[0])}
      />
      {board && <PreferenceDialog board={board} close={() => setBoard(undefined)} />}
    </>
  );
}
function DeleteSelected({ resources }: { resources: DashboardSummaryView[] }) {
  const [boards, setBoards] = useState<DashboardSummaryView[]>();
  return (
    <>
      <ActionButton
        title="Delete"
        iconPosition="left"
        variant="destructive"
        icon={<Trash className="size-4" />}
        disabled={!resources.every((b) => b.canDelete)}
        onClick={() => setBoards(resources)}
      />
      {boards && <DashboardDeleteDialog group boards={boards} close={() => setBoards(undefined)} />}
    </>
  );
}
export const dashboardGroupActions: ButtonGroupComponent<DashboardSummaryView>[] = [
  group.open,
  SetSelectedHome,
  DeleteSelected,
];
