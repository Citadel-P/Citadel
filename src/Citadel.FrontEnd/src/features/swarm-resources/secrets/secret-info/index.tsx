import { SwarmSecretView } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { DockerLabelsSection, Section } from '@/components/custom/common';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { Info } from 'lucide-react';
import { NoResourceActions, ServiceReferences, StaleBadge, StaleWarning } from '../../shared';
import { useSecretInfoGroup } from '../hooks/useSecretsGroup';
import { SecretInfoTable } from './table';

export const SecretInfoComponents: RequiredSwarmInfoComponents<SwarmSecretView> = {
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
          <AlertMessage title="Secret metadata" type="info">
            Only metadata and service references are available. Docker never returns the secret value.
          </AlertMessage>
          <Section title="Details" Icon={Info}>
            <SecretInfoTable secret={resource} />
          </Section>
          <ServiceReferences names={resource.serviceNames} />
          <DockerLabelsSection labels={resource.labels} />
        </div>
      ),
    },
  ],
  useData: useSecretInfoGroup,
};
