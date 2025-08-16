import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { Constants } from '@/lib/constants';

export const useRunImageForm = () => {
  const formSchema = z
    .object({
      name: z.string().optional(),
      labels: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      envVars: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      ports: z.array(z.string()).optional(),
      volumes: z
        .array(
          z
            .object({
              hostPath: z.string(),
              containerPath: z.string(),
            })
            .superRefine((data, ctx) => {
              if (data.containerPath && !data.hostPath) {
                ctx.addIssue({
                  code: z.ZodIssueCode.custom,
                  message: 'Host path is required.',
                  path: ['hostPath'],
                });
              }
              if (data.hostPath && !data.containerPath) {
                ctx.addIssue({
                  code: z.ZodIssueCode.custom,
                  message: 'Container path is required.',
                  path: ['containerPath'],
                });
              }
            }),
        )
        .optional(),
    })
    .refine((data) => !data.name || new RegExp(Constants.validNameIdentifier).test(data.name), {
      message: 'Must be a valid name, no whitespace or special chars are allowed.',
      path: ['name'],
    });

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    mode: 'all',
    defaultValues: {
      name: '',
      labels: [],
      envVars: [],
      ports: [],
      volumes: [],
    },
  });

  return { form };
};
