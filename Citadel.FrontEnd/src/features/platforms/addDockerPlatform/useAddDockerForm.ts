import { Constants } from '@/lib/constants';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';

export const useAddDockerForm = () => {
  const formSchema = z.object({
    name: z.string().min(4, {
      message: 'Platform name must be at least 3 characters.',
    }),
    address: z.string().regex(new RegExp(Constants.validHostOrIp), {
      message: 'Please provide a valid hostname or ip',
    }),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: '',
      address: '',
    },
  });

  return { form };
};
