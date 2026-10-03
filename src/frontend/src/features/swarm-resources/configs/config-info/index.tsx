import { DockerLabelsSection, Section } from '@/components/custom/common';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { Boxes, Info } from 'lucide-react';
import { StaleBadge, StaleWarning } from '../../shared';
import { ReferencingServices } from '../../resource-references';
import { ConfigInfoActions } from '../actions';
import { SwarmConfigInfoView, useConfigInfoGroup } from '../hooks/useConfigsGroup';
import { ConfigInfoTable } from './table';

export const ConfigInspect = ({ resource }: { resource: SwarmConfigInfoView }) => (
  <div className="flex flex-col gap-8">
    <Section title="Details" Icon={Info}>
      <ConfigInfoTable config={resource} />
    </Section>
    {resource.serviceNames.length > 0 && (
      <Section title="Services using this config" Icon={Boxes}>
        <ReferencingServices resource={resource} />
      </Section>
    )}
    <DockerLabelsSection labels={resource.labels} />
  </div>
);

export const ConfigInfoComponents: RequiredSwarmInfoComponents<SwarmConfigInfoView> = {
  Header: {
    Indicator: ({ resource }) => <StateIndicator variant="badge" value={resource.inUse} />,
    NameSuffix: StaleBadge,
    ActionButtons: ({ resource }) => (
      <GenericActionBarButtons resource={resource} actions={Object.values(ConfigInfoActions)} />
    ),
  },
  SubHeader: ({ resource }) => <StaleWarning resource={resource} />,
  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => <ConfigInspect resource={resource} />,
    },
  ],
  useData: useConfigInfoGroup,
};
