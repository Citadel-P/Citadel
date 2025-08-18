import { zodResolver } from '@hookform/resolvers/zod';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { Constants } from '@/lib/constants';
import { ContainerRestartPolicy } from '@/api/_generated';

export const useRunImageForm = () => {
  const formSchema = z
    .object({
      name: z
        .string()
        .regex(new RegExp(Constants.validNameIdentifier), {
          message: 'Must be a valid name, no whitespace or special chars are allowed.',
        })
        .optional()
        .or(z.literal('')),
      workingDir: z.string().optional(),
      user: z.string().optional(),
      labels: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      envVars: z.array(z.object({ key: z.string(), value: z.string() })).optional(),
      ports: z.array(z.string()).optional(),
      autoRemove: z.boolean().optional(),
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
      networks: z.array(z.string()).optional(),
      memoryLimit: z.number().optional(),
      memoryReservation: z.number().optional(),
      cpuLimit: z.number().optional(),
      restartPolicy: z
        .enum([
          ContainerRestartPolicy.No,
          ContainerRestartPolicy.Always,
          ContainerRestartPolicy.UnlessStopped,
          ContainerRestartPolicy.OnFailure,
        ])
        .optional(),
      entryPoint: z.array(z.object({ value: z.string() })).optional(),
      command: z.array(z.object({ value: z.string() })).optional(),
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
      workingDir: '',
      user: '',
      labels: [],
      envVars: [],
      ports: [],
      volumes: [],
      networks: [],
      memoryReservation: 0,
      memoryLimit: 0,
      cpuLimit: 0,
      restartPolicy: ContainerRestartPolicy.No,
      autoRemove: false,
      entryPoint: [],
      command: [],
    },
  });

  return { form };
};
