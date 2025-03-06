import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { JSX, useState } from 'react';
import DockerRegistryConfiguration from './DockerRegistryConfiguration';
import GhcrConfiguration from './GhcrConfiguration';
interface RegistryProvider {
  id: string;
  name: string;
  description: string;
  configuration: JSX.Element;
}
const AddRegistry = () => {
  const [provider, setProvider] = useState<string>('dockerhub');
  const registryProviders: RegistryProvider[] = [
    {
      id: 'dockerhub',
      name: 'DockerHub',
      description: 'Docker hub authenticated account',
      configuration: <DockerRegistryConfiguration />,
    },
    {
      id: 'ghcr',
      name: 'GitHub',
      description: 'GitHub container registry Ghcr',
      configuration: <GhcrConfiguration />,
    },
    {
      id: 'aws',
      name: 'AWS ECR',
      description: 'Amazon elastic container registry',
      configuration: <>Amazon Cfg</>,
    },
    {
      id: 'gitlab',
      name: 'Gitlab',
      description: 'GitLab container registry',
      configuration: <>Gitlab Cfg</>,
    },
    {
      id: 'custom',
      name: 'Custom registry',
      description: 'Define your own registry',
      configuration: <>Custom Cfg</>,
    },
  ] as const;

  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h5 className="text-md font-bold text-foreground">Create registry</h5>
          </div>
        </div>

        <ol className="relative border-s border-border ml-1">
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center dark:text-foreground justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              1
            </span>
            <h2 className="text-sm mb-2 font-semibold text-foreground">Choose a registry provider</h2>
            <RadioGroup
              defaultValue="dockerhub"
              className="flex  flex-wrap justify-between"
              onValueChange={(v) => setProvider(v)}>
              {registryProviders.map((provider) => (
                <Label
                  key={provider.id}
                  className={`hover:bg-accent/50 hover:cursor-pointer flex-grow flex-1 flex items-start gap-3 rounded-lg border p-4 has-[[data-state=checked]]:border-primary/50 has-[[data-state=checked]]:bg-primary/10`}>
                  <RadioGroupItem
                    value={provider.id}
                    id={provider.name}
                    className="shadow-none data-[state=checked]:border-primary/50 data-[state=checked]:bg-primary *:data-[slot=radio-group-indicator]:[&>svg]:fill-white *:data-[slot=radio-group-indicator]:[&>svg]:stroke-white"
                  />
                  <div className="grid gap-1 font-normal">
                    <div className="font-medium">{provider.name}</div>
                    <div className="text-muted-foreground leading-snug">{provider.description}</div>
                  </div>
                </Label>
              ))}
            </RadioGroup>
          </li>
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center dark:text-foreground justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              2
            </span>
            <h2 className="text-sm font-semibold text-foreground">Configure</h2>
            {registryProviders.find((s) => s.id === provider)?.configuration}
          </li>
        </ol>
      </div>
    </div>
  );
};

export default AddRegistry;
