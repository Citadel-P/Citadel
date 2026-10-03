import { useNavigate, useParams } from 'react-router';
import { toast } from 'sonner';
import { useMutate } from '@/lib/hooks';
import { RequiredFormComponents } from '@/pages/types';
import { SwarmDataResourceForm } from '../resource-form';

export const SecretForm = () => {
  const navigate = useNavigate();
  const { platformId = '' } = useParams<{ platformId: string }>();
  const mutation = useMutate('createSwarmSecret', { gcTime: 0 });

  return (
    <SwarmDataResourceForm
      kind="secret"
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
        toast.success(`Secret ${value.name} created.`);
        navigate(`/platforms/${platformId}/secrets`);
      }}
    />
  );
};

export const SecretFormComponents: RequiredFormComponents = {
  AddForm: {
    Header: { title: 'Secret' },
    Content: SecretForm,
  },
  EditForm: undefined,
};
