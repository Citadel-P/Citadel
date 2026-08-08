import { useState } from 'react';
import { Pencil } from 'lucide-react';
import { useParams } from 'react-router';
import { toast } from 'sonner';
import { SwarmNodeView } from '@/api/generated/api.types';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { Button } from '@/components/ui/button';
import { useMutate } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { DropdownActionComponent } from '@/pages/types';
import { SwarmLabelsEditDialog } from '../resource-edit-dialog';

export const NodeEditDropdownAction = ({
  resource,
  onAction,
}: React.ComponentProps<DropdownActionComponent<SwarmNodeView>>) => (
  <DropdownActionButton
    title="Edit"
    icon={<Pencil className="h-4 w-4" />}
    disabled={resource.isStale || !hasCapability(resource, 'canWrite')}
    onClick={() => onAction?.('edit')}
  />
);

export const NodeEditInfoAction = ({ resource }: { resource: SwarmNodeView }) => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button
        variant="outline"
        size="sm"
        disabled={resource.isStale || !hasCapability(resource, 'canWrite')}
        onClick={() => setOpen(true)}>
        <Pencil className="h-3.5 w-3.5" /> Edit
      </Button>
      {open && <NodeEditDialog resource={resource} open onOpenChange={setOpen} />}
    </>
  );
};

export const NodeEditDialog = ({
  resource,
  open,
  onOpenChange,
}: {
  resource: SwarmNodeView;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [versionIndex] = useState(resource.versionIndex);
  const mutation = useMutate('updateSwarmNode');

  const save = async (labels: Record<string, string>) => {
    await mutation.mutateAsync({
      platformId,
      nodeId: resource.id,
      data: {
        versionIndex,
        availability: resource.availability,
        labels,
      },
    });
    toast.success(`Node ${resource.hostname} updated.`);
    onOpenChange(false);
  };

  return (
    <SwarmLabelsEditDialog
      title="Edit node"
      description="Edit the labels assigned to this Node. Use the availability actions on the Node page to change scheduling."
      initialLabels={resource.labels}
      open={open}
      pending={mutation.isPending}
      onOpenChange={onOpenChange}
      onSave={save}
    />
  );
};
