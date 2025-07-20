import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { Constants } from '@/lib/constants';

export const useVolumeForm = () => {
  const formSchema = z.object({
    name: z
      .string()
      .regex(new RegExp(Constants.validNameIdentifier), {
        message: 'Must be a valid name, no whitespace or special chars are allowed.',
      })
      .min(4, {
        message: 'Name must be at least 4 characters.',
      }),
    driver: z.string(),
    options: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
    labels: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
  });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    mode: 'all',
    defaultValues: {
      name: '',
      driver: 'local',
      options: [],
      labels: [],
    },
  });

  return { form };
};
