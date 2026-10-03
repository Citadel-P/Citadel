import { useNavigate, useParams } from 'react-router';
import { toast } from 'sonner';
import { useMutate } from '@/lib/hooks';
import { RequiredFormComponents } from '@/pages/types';
import { SwarmDataResourceForm } from '../resource-form';

export const ConfigForm = () => {
  const navigate = useNavigate();
  const { platformId = '' } = useParams<{ platformId: string }>();
  const mutation = useMutate('createSwarmConfig');

  return (
    <SwarmDataResourceForm
      kind="config"
      pending={mutation.isPending}
      onCreate={async (value) => {
        await mutation.mutateAsync({
          platformId,
          data: {
            name: value.name,
            data: value.data,
            labels: Object.fromEntries(value.labels.map((label) => [label.key, label.value])),
          },
        });
        toast.success(`Config ${value.name} created.`);
        navigate(`/platforms/${platformId}/configs`);
      }}
    />
  );
};

export const ConfigFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: { title: 'Config' },
    Content: ConfigForm,
  },
  EditForm: undefined,
};
