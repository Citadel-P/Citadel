import { RegistryConfigurationBaseGitHubRegistry, RegistryType } from '@/api/generated/api.types';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { useRegistryFormContext } from '../registry-form-context';
import { Constants } from '@/lib/constants';

export const useGhcrForm = () => {
  const { registry } = useRegistryFormContext();
  const configuration = registry?.configuration as RegistryConfigurationBaseGitHubRegistry;

  const formSchema = z.object({
    name: z
      .string()
      .regex(new RegExp(Constants.validNameIdentifier), {
        message: 'Must be a valid name, no whitespace or special chars are allowed.',
      })
      .min(5, {
        message: 'Name must be at least 5 characters.',
      }),
    type: z.string(),
    url: z.string(),
    configuration: z.object({
      $type: z.string(),
      name: z.string().min(5),
      pat: z.string().min(10),
      type: z.string(),
    }),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    mode: 'all',
    values: {
      name: registry?.name ?? '',
      url: 'https://ghcr.io',
      type: RegistryType.GitHub,
      configuration: {
        $type: RegistryType.GitHub,
        name: configuration?.name ?? '',
        pat: configuration?.pat ?? '',
        type: configuration?.type ?? 'Organization',
      },
    },
  });

  return { form };
};
