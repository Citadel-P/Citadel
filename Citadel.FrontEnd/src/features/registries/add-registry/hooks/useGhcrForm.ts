import { RegistryDiscriminator } from '@/api/_generated';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';

export const useGhcrForm = () => {
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
      name: '',
      url: 'https://ghcr.io',
      discriminator: RegistryDiscriminator.GitHub,
      configuration: {
        $type: RegistryDiscriminator.GitHub,
        name: '',
        pat: '',
        type: 'Organization',
      },
    },
  });

  return { form };
};
