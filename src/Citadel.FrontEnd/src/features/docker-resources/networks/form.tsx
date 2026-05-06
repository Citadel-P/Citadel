import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { useMutate } from '@/lib/hooks';
import { useAppContext } from '@/lib/context/app-context';
import { Constants } from '@/lib/constants';

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import {
  FormShell,
  defineSection,
  defineField,
  defineGroupField,
  FieldInput,
  FieldSwitch,
} from '@/components/custom/form-builder';
import { KeyValuePairInput, KVPair } from '@/components/custom/key-value-pair-input';

import { CreateNetworkInput, IPAMConfigInput } from '@/api/generated/api.types';

type CreateNetworkFormInput = Omit<
  CreateNetworkInput,
  'platformId' | 'scope' | 'configOnly' | 'labels' | 'options' | 'ipam'
> & {
  labels: KVPair[];
  options: KVPair[];
  ipam: {
    driver: string;
    config: Partial<IPAMConfigInput>[];
  };
};

const DRIVER_OPTIONS = [
  { value: 'bridge', label: 'Bridge' },
  { value: 'overlay', label: 'Overlay' },
  { value: 'ipvlan', label: 'Ipvlan' },
  { value: 'macvlan', label: 'Macvlan' },
];

const REGEX = {
  ipv4Cidr: /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}\/([0-9]|[1-2][0-9]|3[0-2])$/,
  ipv4: /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/,
  ipv6Cidr:
    /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|:(:[0-9a-fA-F]{1,4}){1,7}|([0-9a-fA-F]{1,4}:){1,6}(:[0-9a-fA-F]{1,4}){1}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){5}|([0-9a-fA-F]{1,4}:){1}(:[0-9a-fA-F]{1,4}){6}|:((:[0-9a-fA-F]{1,4}){1,7}|:))\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/,
  ipv6: /^([a-fA-F0-9:]+:+)+[a-fA-F0-9]+$/,
};

export default function AddNetwork({ mode }: { mode: 'add' | 'edit' }) {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const { mutateAsync, isPending } = useMutate('createNetwork');

  const defaultValues: CreateNetworkFormInput = {
    name: '',
    driver: 'bridge',
    enableIPv4: true,
    enableIPv6: false,
    internal: false,
    attachable: false,
    ingress: false,
    labels: [],
    options: [],
    ipam: {
      driver: 'default',
      config: [{}, {}],
    },
  };

  const [update, setUpdate] = useState<Partial<CreateNetworkFormInput>>(defaultValues);
  const original = defaultValues;

  const merged = useMemo(() => {
    return { ...original, ...update, ipam: { ...original.ipam, ...update.ipam } } as CreateNetworkFormInput;
  }, [original, update]);

  const onSave = async (formValues: CreateNetworkFormInput) => {
    if (!formValues.enableIPv4 && !formValues.enableIPv6) {
      toast.error('At least one of IPv4 or IPv6 must be enabled.');
      return;
    }

    const optionsObj = Object.fromEntries((formValues.options ?? []).map(({ key, value }) => [key, value]));
    const labelsObj = Object.fromEntries((formValues.labels ?? []).map(({ key, value }) => [key, value]));

    const ipamConfig = [
      formValues.enableIPv4 && formValues.ipam?.config?.[0] ? formValues.ipam.config[0] : {},
      formValues.enableIPv6 && formValues.ipam?.config?.[1] ? formValues.ipam.config[1] : {},
    ];

    const payload: CreateNetworkInput = {
      ...formValues,
      platformId: currentPlatform?.id ?? '',
      scope: 'local',
      configOnly: false,
      labels: labelsObj,
      options: optionsObj,
      ipam: {
        driver: formValues.ipam?.driver ?? 'default',
        config: ipamConfig as IPAMConfigInput[],
      },
    };

    const result = await mutateAsync(payload);

    if (result?.data) {
      toast.success(`Network created (ID: ${result.data.id})`);
      navigate(`/platforms/${currentPlatform?.id}/networks`);
    }
  };

  const schema = {
    basic: defineSection<CreateNetworkFormInput>({
      title: 'Basic Configuration',
      items: [
        defineField({
          key: 'name',
          label: 'Name',
          description: "The network's name.",
          required: true,
          placeholder: 'e.g. my-network',
          validate: (v) =>
            !new RegExp(Constants.validNameIdentifier).test(v)
              ? 'Must be a valid name'
              : v && v.length < 4
                ? 'Name must be at least 4 characters.'
                : null,
          render: (value, set) => <FieldInput value={value ?? ''} onChange={(v) => set({ name: v })} />,
        }),
        defineField({
          key: 'driver',
          label: 'Driver',
          description: 'Name of the network driver plugin to use.',
          required: true,
          render: (value, set) => (
            <Select onValueChange={(v) => set({ driver: v })} value={value}>
              <SelectTrigger className="w-full max-w-100">
                <SelectValue placeholder="Select driver" />
              </SelectTrigger>
              <SelectContent className="bg-background">
                {DRIVER_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ),
        }),
      ],
    }),
    advanced: defineSection<CreateNetworkFormInput>({
      title: 'Advanced (Optional)',
      items: [
        defineGroupField<CreateNetworkFormInput>({
          id: 'ipv4',
          label: 'IPv4 Configuration',
          items: [
            defineField({
              key: 'enableIPv4',
              label: 'Enable IPv4',
              description: 'Enable IPv4 on the network.',
              render: (value, set) => (
                <FieldSwitch id="enableIPv4" checked={!!value} onChange={(v) => set({ enableIPv4: v })} />
              ),
            }),

            ...(merged.enableIPv4
              ? [
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.0.subnet',
                    label: 'IPv4 Subnet',
                    validate: (v) =>
                      v && !REGEX.ipv4Cidr.test(v.trim()) ? 'Invalid IPv4 subnet (e.g. 192.168.1.0/24)' : null,
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. 192.168.1.0/24"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                0: { ...prev.ipam?.config?.[0], subnet: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.0.gateway',
                    label: 'IPv4 Gateway',
                    validate: (v) =>
                      v && !REGEX.ipv4.test(v.trim()) ? 'Invalid IPv4 gateway (e.g. 192.168.1.1)' : null,
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. 192.168.1.1"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                0: { ...prev.ipam?.config?.[0], gateway: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.0.ipRange',
                    label: 'IPv4 Range',
                    validate: (v) =>
                      v && !REGEX.ipv4Cidr.test(v.trim()) ? 'Invalid IPv4 range (e.g. 192.168.1.0/25)' : null,
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. 192.168.1.0/25"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                0: { ...prev.ipam?.config?.[0], ipRange: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                ]
              : []),
          ],
        }),

        defineGroupField<CreateNetworkFormInput>({
          id: 'ipv6',
          label: 'IPv6 Configuration',
          items: [
            defineField({
              key: 'enableIPv6',
              label: 'Enable IPv6',
              description: 'Enable IPv6 on the network.',
              render: (value, set) => (
                <FieldSwitch id="enableIPv6" checked={!!value} onChange={(v) => set({ enableIPv6: v })} />
              ),
            }),
            ...(merged.enableIPv6
              ? [
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.1.subnet',
                    label: 'IPv6 Subnet',
                    validate: (v) =>
                      v && !REGEX.ipv6Cidr.test(v.trim()) ? 'Invalid IPv6 subnet (e.g. fd00::/64)' : null,
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. fd00::/64"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                1: { ...prev.ipam?.config?.[1], subnet: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.1.gateway',
                    label: 'IPv6 Gateway',

                    validate: (v) => (v && !REGEX.ipv6.test(v.trim()) ? 'Invalid IPv6 gateway (e.g. fd00::1)' : null),
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. fd00::1"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                1: { ...prev.ipam?.config?.[1], gateway: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                  defineField<CreateNetworkFormInput, any>({
                    key: 'ipam.config.1.ipRange',
                    label: 'IPv6 Range',
                    validate: (v) =>
                      v && !REGEX.ipv6Cidr.test(v.trim()) ? 'Invalid IPv6 range (e.g. fd00::/64)' : null,
                    render: (value, set) => (
                      <FieldInput
                        value={value}
                        placeholder="e.g. fd00::/64"
                        onChange={(v) =>
                          set((prev) => ({
                            ipam: {
                              driver: prev.ipam?.driver ?? 'default',
                              config: Object.assign([], prev.ipam?.config, {
                                1: { ...prev.ipam?.config?.[1], ipRange: v },
                              }),
                            },
                          }))
                        }
                      />
                    ),
                  }),
                ]
              : []),
          ],
        }),

        defineField({
          key: 'labels',
          label: 'Labels',
          description: 'User-defined key/value metadata.',
          render: (value, set) => (
            <KeyValuePairInput
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
          description: 'Network specific options to be used by the drivers.',
          render: (value, set) => (
            <KeyValuePairInput
              value={value ?? []}
              onChange={(next) => set({ options: next })}
              addButtonLabel="Add driver option"
            />
          ),
        }),
        defineField({
          key: 'internal',
          label: 'Internal',
          description: 'Restrict external access to and from this network.',
          render: (value, set) => (
            <FieldSwitch id="internalNetwork" checked={!!value} onChange={(v) => set({ internal: v })} />
          ),
        }),
        defineField({
          key: 'attachable',
          label: 'Attachable',
          description: 'Controls which types of containers can connect to an overlay network.',
          render: (value, set) => (
            <FieldSwitch id="attachableNetwork" checked={!!value} onChange={(v) => set({ attachable: v })} />
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
      draftKey={`network:new:${currentPlatform?.id}`}
      draftVersion={1}
      pending={isPending}
    />
  );
}
