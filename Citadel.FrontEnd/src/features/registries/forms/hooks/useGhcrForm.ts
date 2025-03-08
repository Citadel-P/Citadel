import { IRegistryConfigurationGitHubRegistry, RegistryDiscriminator } from '@/api/_generated';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { useContextSelector } from 'use-context-selector';
import { z } from 'zod';
import { RegistryFormContext } from '../RegistryFormProvider';

export const useGhcrForm = () => {
  const registry = useContextSelector(RegistryFormContext, (v) => v?.registry);
  const configuration = registry?.configuration as IRegistryConfigurationGitHubRegistry;
  
  const formSchema = z.object({
    name: z.string().min(5, {
      message: 'Name must be at least 5 characters.',
    }),
    discriminator: z.string(),
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
    defaultValues: {
      name: registry?.name ?? '',
      url: 'https://ghcr.io',
      discriminator: RegistryDiscriminator.GitHub,
      configuration: {
        $type: RegistryDiscriminator.GitHub,
        name: configuration?.name ?? '',
        pat: configuration?.pat ?? '',
        type: configuration?.type ?? 'Organization',
      },
    },
  });

  return { form };
};
