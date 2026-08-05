import { SwarmConfigView } from '@/api/generated/api.types';
import { DockerLabelsSection, Section } from '@/components/custom/common';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { Info } from 'lucide-react';
import { NoResourceActions, ServiceReferences, StaleBadge, StaleWarning } from '../../shared';
import { useConfigInfoGroup } from '../hooks/useConfigsGroup';
import { ConfigInfoTable } from './table';

export const ConfigInfoComponents: RequiredSwarmInfoComponents<SwarmConfigView> = {
  Header: {
    NameSuffix: StaleBadge,
    ActionButtons: NoResourceActions,
  },
  SubHeader: ({ resource }) => <StaleWarning resource={resource} />,
  Tabs: [
    {
      label: 'Inspect',
      Content: ({ resource }) => (
        <div className="flex flex-col gap-8">
          <Section title="Details" Icon={Info}>
            <ConfigInfoTable config={resource} />
          </Section>
          <ServiceReferences names={resource.serviceNames} />
          <DockerLabelsSection labels={resource.labels} />
        </div>
      ),
    },
  ],
  useData: useConfigInfoGroup,
};
