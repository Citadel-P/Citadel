import { DockerLabelsSection, Section } from '@/components/custom/common';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Boxes, Info } from 'lucide-react';
import { StaleBadge, StaleWarning } from '../../shared';
import { ReferencingServices } from '../../resource-references';
import { SecretInfoActions } from '../actions';
import { SwarmSecretInfoView, useSecretInfoGroup } from '../hooks/useSecretsGroup';
import { SecretInfoTable } from './table';

export const SecretInspect = ({ resource }: { resource: SwarmSecretInfoView }) => (
  <div className="flex flex-col gap-8">
    <Section title="Details" Icon={Info}>
      <SecretInfoTable secret={resource} />
    </Section>
    {resource.serviceNames.length > 0 && (
      <Section title="Services using this secret" Icon={Boxes}>
        <ReferencingServices resource={resource} />
      </Section>
    )}
    <DockerLabelsSection labels={resource.labels} />
  </div>
);

export const SecretInfoComponents: RequiredSwarmInfoComponents<SwarmSecretInfoView> = {
  Header: {
    Indicator: ({ resource }) => <StateIndicator value={resource.inUse} />,
    NameSuffix: StaleBadge,
    ActionButtons: ({ resource }) => (
      <GenericActionBarButtons resource={resource} actions={Object.values(SecretInfoActions)} />
    ),
  },
  SubHeader: ({ resource }) => <StaleWarning resource={resource} />,
  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <SecretInspect resource={resource} />,
    },
  ],
  useData: useSecretInfoGroup,
};
