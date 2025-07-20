import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { Constants } from '@/lib/constants';

export const useNetworkForm = () => {
  const ipv4CidrRegex =
    /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}\/([0-9]|[1-2][0-9]|3[0-2])$/;
  const ipv4Regex = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;
  const ipv6CidrRegex = /^([a-fA-F0-9:]+:+)+[a-fA-F0-9]+\/\d{1,3}$/;
  const ipv6Regex = /^([a-fA-F0-9:]+:+)+[a-fA-F0-9]+$/;

  const formSchema = z
    .object({
      name: z
        .string()
        .regex(new RegExp(Constants.validNameIdentifier), {
          message: 'Must be a valid name, no whitespace or special chars are allowed.',
        })
        .min(4, {
          message: 'Name must be at least 4 characters.',
        }),
      driver: z.string(),
      enableIPv4: z.boolean(),
      enableIPv6: z.boolean(),
      internal: z.boolean(),
      attachable: z.boolean(),
      ingress: z.boolean(),
      options: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      labels: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      ipam: z.object({
        driver: z.string().optional().nullable(),
        config: z.array(
          z.object({
            subnet: z.string().optional(),
            ipRange: z.string().optional(),
            gateway: z.string().optional(),
          }),
        ),
      }),
    })
    .superRefine((data, ctx) => {
      const v4 = data.ipam?.config?.[0];
      const v6 = data.ipam?.config?.[1];

      if (data.ipam?.config?.length > 2) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'Only two IPAM config entries are allowed (IPv4 and IPv6).',
          path: ['ipam', 'config'],
        });
      }

      if (data.enableIPv4 && v4) {
        if (v4.subnet && !ipv4CidrRegex.test(v4.subnet.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv4 subnet (e.g. 192.168.1.0/24)',
            path: ['ipam', 'config', 0, 'subnet'],
          });
        }
        if (v4.ipRange && !ipv4CidrRegex.test(v4.ipRange.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv4 range (e.g. 192.168.1.0/25)',
            path: ['ipam', 'config', 0, 'ipRange'],
          });
        }
        if (v4.gateway && !ipv4Regex.test(v4.gateway.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv4 gateway (e.g. 192.168.1.1)',
            path: ['ipam', 'config', 0, 'gateway'],
          });
        }
      }
      // Only validate format if subnet is present for IPv6
      if (data.enableIPv6 && v6) {
        if (v6.subnet && !ipv6CidrRegex.test(v6.subnet.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv6 subnet (e.g. fd00::/64)',
            path: ['ipam', 'config', 1, 'subnet'],
          });
        }
        if (v6.ipRange && !ipv6CidrRegex.test(v6.ipRange.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv6 range (e.g. fd00::1/64)',
            path: ['ipam', 'config', 1, 'ipRange'],
          });
        }
        if (v6.gateway && !ipv6Regex.test(v6.gateway.trim())) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Invalid IPv6 gateway (e.g. fd00::1)',
            path: ['ipam', 'config', 1, 'gateway'],
          });
        }
      }
      // At least one of IPv4 or IPv6 must be enabled
      if (!data.enableIPv4 && !data.enableIPv6) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: 'At least one of IPv4 or IPv6 must be enabled.',
          path: ['enableIPv4'],
        });
      }
    });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    mode: 'all',
    defaultValues: {
      name: '',
      driver: 'bridge',
      enableIPv4: true,
      enableIPv6: false,
      internal: false,
      attachable: false,
      ingress: false,
      options: [],
      labels: [],
      ipam: {
        driver: 'default',
        config: [
          {
            subnet: '',
            ipRange: '',
            gateway: '',
          },
          {
            subnet: '',
            ipRange: '',
            gateway: '',
          },
        ],
      },
    },
  });

  return { form };
};
