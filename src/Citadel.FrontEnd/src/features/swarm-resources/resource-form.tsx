import { useMemo, useState } from 'react';
import { FormShell, FieldInput, defineField, defineSection } from '@/components/custom/form-builder';
import { KeyValuePairInput, KVPair } from '@/components/custom/key-value-pair-input';
import { Constants } from '@/lib/constants';
import { MonacoEditor } from '@/lib/monaco';

export type SwarmDataResourceFormValue = {
  name: string;
  data: string;
  labels: KVPair[];
};

export const SwarmDataResourceForm = ({
  kind,
  pending,
  onCreate,
}: {
  kind: 'secret' | 'config';
  pending: boolean;
  onCreate: (value: SwarmDataResourceFormValue) => Promise<void>;
}) => {
  const original = useMemo<SwarmDataResourceFormValue>(() => ({ name: '', data: '', labels: [] }), []);
  const [update, setUpdate] = useState<Partial<SwarmDataResourceFormValue>>({});
  const isSecret = kind === 'secret';
  const maxBytes = isSecret ? 500 * 1024 : 1000 * 1024;

  const schema = useMemo(
    () => ({
      resource: defineSection<SwarmDataResourceFormValue>({
        title: 'Configuration',
        items: [
          defineField({
            key: 'name',
            label: 'Name',
            description: `The Swarm ${kind} name.`,
            required: true,
            validate: (value) =>
              !new RegExp(Constants.validNameIdentifier).test(value ?? '')
                ? 'Must be a valid name without whitespace or special characters.'
                : null,
            render: (value, set) => (
              <FieldInput
                value={value ?? ''}
                placeholder={`e.g. application-${kind}`}
                onChange={(name) => set({ name })}
              />
            ),
          }),
          defineField({
            key: 'data',
            label: isSecret ? 'Secret value' : 'Config data',
            description: isSecret
              ? 'Sent directly to the Swarm manager. Citadel does not persist or return this value.'
              : 'The content made available to services that reference this config.',
            required: isSecret,
            validate: (value) =>
              new TextEncoder().encode(value ?? '').byteLength > maxBytes
                ? `${isSecret ? 'Secret' : 'Config'} data exceeds the ${isSecret ? '500 KiB' : '1000 KiB'} limit.`
                : null,
            render: (value, set) => (
              <MonacoEditor
                value={value ?? ''}
                language="plaintext"
                minHeight={240}
                className="mx-0 my-0"
                onValueChange={(data) => set({ data })}
              />
            ),
          }),
          defineField({
            key: 'labels',
            label: 'Labels',
            description: 'Optional key/value metadata.',
            render: (value, set) => (
              <KeyValuePairInput
                label="Labels"
                value={value ?? []}
                onChange={(labels) => set({ labels })}
                addButtonLabel="Add label"
              />
            ),
          }),
        ],
      }),
    }),
    [isSecret, kind, maxBytes],
  );

  return (
    <FormShell
      mode="add"
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      pending={pending}
      saveLabel={`Create ${kind}`}
      onSave={async (value) => {
        await onCreate(value);
        setUpdate({ data: '' });
      }}
    />
  );
};
