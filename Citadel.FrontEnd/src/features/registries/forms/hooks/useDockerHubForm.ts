import { RegistryConfigurationBaseDockerHubRegistry, RegistryType } from '@/api/_generated';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { RegistryFormContext } from '../RegistryFormProvider';
import { useContextSelector } from 'use-context-selector';
import { Constants } from '@/lib/constants';

export const useDockerHubForm = () => {
  const registry = useContextSelector(RegistryFormContext, (v) => v?.registry);
  const configuration = registry?.configuration as RegistryConfigurationBaseDockerHubRegistry;
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
      username: z.string().min(5),
      pat: z.string().min(10),
    }),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    values: {
      name: registry?.name ?? '',
      url: 'https://hub.docker.com',
      type: RegistryType.DockerHub,
      configuration: {
        $type: RegistryType.DockerHub,
        username: configuration?.userName ?? '',
        pat: configuration?.pat ?? '',
      },
    },
  });

  return { form };
};
