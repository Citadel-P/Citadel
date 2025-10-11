import Loader from '@/components/ui/loader';
import { useDeploymentFormContext } from './DeploymentFormContext';
import { Config, ConfigItem } from '@/components/dsl/config';
import { DeploymentSpec, DeploymentVersion, DeploymentView } from '@/api/generated/api.types';
import { ResourceSelector } from '@/components/dsl/common';
import { useLocalStorage } from '@/lib/hooks';

const DeploymentForm = () => {
  const { formTitle, isLoading } = useDeploymentFormContext();

  if (isLoading) return <Loader />;
  return (
    <div className="mx-auto px-4 py-3 lg:container sm:px-6">
      <div className="w-full rounded-lg border-border bg-background p-4">
        <h5 className="text-md font-bold text-foreground">{formTitle}</h5>
        <DeploymentConfig />
      </div>
    </div>
  );
};

const DeploymentConfig = () => {
  const deployment = {
    id: '1',
    name: 'dep1',
    versions: [
      {
        version: 1,
        createdBy: 'piter',
        spec: {
          imageId: '1234',
          target: {
            platformId: 'p01',
            replicas: 2,
          },
        } as DeploymentSpec,
      },
    ] as DeploymentVersion[],
  } as DeploymentView;
  const [update, set] = useLocalStorage<Partial<DeploymentView>>(`deployment-create`, {});
  const platforms = [
    { id: '1', name: 'p01' },
    { id: '2', name: 'p02' },
    { id: '3', name: 'p03' },
  ];
  return (
    <Config
      titleOther={'hello'}
      disabled={false}
      original={deployment}
      update={{}}
      set={set}
      onSave={async () => {}}
      components={{
        '': [
          {
            label: 'Name',
            labelHidden: true,
            components: {
              name: (name, set) => {
                return (
                  <ConfigItem
                    label={name ? <div className="flex gap-3 text-lg font-bold">Name: {name}</div> : 'Enter a name'}
                    description="Type the deployment name.">
                    <ResourceSelector
                      selected={name}
                      onSelect={(name) => set({ name })}
                      disabled={false}
                      align="start"
                      items={platforms}
                    />
                  </ConfigItem>
                );
              },
            },
          },

          {
            label: 'Network',
            labelHidden: true,
            components: {
              network: (value, set) => <>NetworkSelector here</>,
            },
          },
        ],
        advanced: [
          {
            label: 'Command',
            labelHidden: true,
            components: {
              command: (value, set) => (
                <ConfigItem
                  label="Command"
                  boldLabel
                  description={
                    <div className="flex flex-row flex-wrap gap-2">
                      <div>Replace the CMD, or extend the ENTRYPOINT.</div>
                    </div>
                  }></ConfigItem>
              ),
            },
          },
          {
            label: 'Labels',
            description: 'Attach --labels to the container.',
            components: {
              labels: (labels, set) => <>Labels here </>,
            },
          },
        ],
      }}
    />
  );
};

export default DeploymentForm;
