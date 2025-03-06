import { RegistryDiscriminator } from '@/api/_generated';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';

export const useDockerHubRegistryForm = () => {
  const formSchema = z.object({
    name: z.string().min(5, {
      message: 'Name must be at least 5 characters.',
    }),
    discriminator: z.string(),
    url: z.string(),
    configuration: z.object({
      $type: z.string(),
      username: z.string().min(5),
      pat: z.string().min(10),
    }),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: '',
      url: 'https://hub.docker.com',
      discriminator: RegistryDiscriminator.DockerHub,
      configuration: {
        $type: RegistryDiscriminator.DockerHub,
        username: '',
        pat: '',
      },
    },
  });

  return { form };
};
