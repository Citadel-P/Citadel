import { useState } from 'react';
import { useAppContext } from '@/lib/context/app-context';
import { useMutate } from '@/lib/hooks';
import { toast } from 'sonner';
import { useNavigate } from 'react-router';

import { FormShell, defineSection, defineField } from '@/components/custom/form-builder';

import { FieldInput } from '@/components/custom/form-builder';
import { KeyValuePairInput } from '@/components/custom/key-value-pair-input';

import { CreateVolumeInput } from '@/api/generated/api.types';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Constants } from '@/lib/constants';

const driverOptions = [{ value: 'local', label: 'Local' }];

export default function VolumeForm({ mode }: { mode: 'add' | 'edit' }) {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const { mutateAsync, isPending } = useMutate('createVolume');

  const [update, setUpdate] = useState<Partial<CreateVolumeInput>>({
    driver: 'local',
    platformId: currentPlatform?.id ?? '',
  });

  const original = {} as CreateVolumeInput;

  const onSave = async (merged: CreateVolumeInput) => {
    const payload = {
      ...merged,
      platformId: currentPlatform?.id ?? '',
      labels: Object.fromEntries((merged.labels ?? []).map((x) => [x.key, x.value])),
      options: Object.fromEntries((merged.options ?? []).map((x) => [x.key, x.value])),
    };

    const result = await mutateAsync(payload);

    if (result?.data) {
      toast.success(`Volume created (ID: ${result.data.id})`);
      navigate(`/platforms/${currentPlatform?.id}/volumes`);
    }
  };

  const schema = {
    basic: defineSection<CreateVolumeInput>({
      title: 'Basic Configuration',
      items: [
        defineField({
          key: 'name',
          label: 'Name',
          description: `The new volume's name. If not specified, Docker generates a name.`,
          validate: (v) =>
            !new RegExp(Constants.validNameIdentifier).test(v)
              ? 'Must be a valid name, no whitespace or special chars are allowed.'
              : null,
          required: false,
          placeholder: 'e.g. my-volume',
          render: (value, set) => (
            <FieldInput
              placeholder="e.g. my-volume"
              value={value ?? ''}
              onChange={(v) => set({ name: v == '' ? undefined : v })}
            />
          ),
        }),

        defineField({
          key: 'driver',
          label: 'Driver',
          required: true,
          render: (value, set) => (
            <Select
              onValueChange={(v) =>
                set(() => ({
                  driver: v,
                }))
              }
              value={value}>
              <SelectTrigger className="w-full max-w-[400px]">
                <SelectValue placeholder="Select driver type" />
              </SelectTrigger>

              <SelectContent className="bg-background">
                {driverOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ),
        }),
      ],
    }),

    advanced: defineSection<CreateVolumeInput>({
      title: 'Advanced (Optional)',
      items: [
        defineField({
          key: 'labels',
          label: 'Labels',
          render: (value, set) => (
            <KeyValuePairInput
              label="User-defined key/value metadata."
              value={value ?? []}
              onChange={(next) => set({ labels: next })}
              addButtonLabel="Add label"
              keyPlaceHolder="com.example.foo"
              valuePlaceHolder="bar"
            />
          ),
        }),

        defineField({
          key: 'options',
          label: 'Driver Options',
          render: (value, set) => (
            <KeyValuePairInput
              label="A mapping of driver options and values"
              value={value ?? []}
              onChange={(next) => set({ options: next })}
              addButtonLabel="Add driver option"
              keyPlaceHolder="type"
              valuePlaceHolder="nfs"
              helpText="Handles how the volume's storage is managed on the underlying system or a remote storage provider, such as NFS, CIFS, or cloud services."
            />
          ),
        }),
      ],
    }),
  };

  if (mode === 'edit') {
    return 'This resource does not allow editing';
  }

  return (
    <FormShell
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={onSave}
      mode="add"
      draftKey={`volume:new:${currentPlatform?.id}`}
      draftVersion={1}
      pending={isPending}
    />
  );
}
