import { lazy } from 'react';
import type { DockerResourceType, ResourceType, SwarmResourceType } from '@/api/types';
import type { RequiredComponents, RequiredDockerInfoComponents, RequiredFormComponents } from '@/pages/types';
import type { ResourceViewProps } from '@/pages/resource-view';
import type { ResourceFormViewProps } from '@/pages/resource-form-view';
import type { ResourceInfoViewProps } from '@/pages/resource-info';

const list = (load: () => Promise<RequiredComponents>) =>
  lazy(async () => {
    const [Components, { ResourceView }] = await Promise.all([load(), import('@/pages/resource-view')]);
    return {
      default: (props: Omit<ResourceViewProps, 'Components'>) => <ResourceView {...props} Components={Components} />,
    };
  });
const form = (load: () => Promise<RequiredFormComponents>) =>
  lazy(async () => {
    const [Components, { ResourceFormView }] = await Promise.all([load(), import('@/pages/resource-form-view')]);
    return {
      default: (props: Omit<ResourceFormViewProps, 'Components'>) => (
        <ResourceFormView {...props} Components={Components} />
      ),
    };
  });
const detail = (load: () => Promise<RequiredDockerInfoComponents>) =>
  lazy(async () => {
    const [Components, { ResourceInfoView }] = await Promise.all([load(), import('@/pages/resource-info')]);
    return {
      default: (props: Omit<ResourceInfoViewProps, 'Components'>) => (
        <ResourceInfoView {...props} Components={Components} />
      ),
    };
  });

export const SwarmResourcePages: Record<SwarmResourceType, ReturnType<typeof list>> = {
  Node: list(() => import('./swarm-resources/nodes').then((m) => m.NodeComponents)),
  Service: list(() => import('./swarm-resources/services').then((m) => m.ServiceComponents)),
  Task: list(() => import('./swarm-resources/tasks').then((m) => m.TaskComponents)),
  Secret: list(() => import('./swarm-resources/secrets').then((m) => m.SecretComponents)),
  Config: list(() => import('./swarm-resources/configs').then((m) => m.ConfigComponents)),
};

export const ResourcePages: Partial<Record<ResourceType, ReturnType<typeof list>>> = {
  Image: list(() => import('./docker-resources/images').then((m) => m.ImageComponents)),
  Volume: list(() => import('./docker-resources/volumes').then((m) => m.VolumeComponents)),
  Network: list(() => import('./docker-resources/networks').then((m) => m.NetworkComponents)),
  Container: list(() => import('./docker-resources/containers').then((m) => m.ContainerComponents)),
  ...SwarmResourcePages,
  Platform: list(() => import('./platforms').then((m) => m.PlatformComponents)),
  Registry: list(() => import('./registries').then((m) => m.RegistryComponents)),
  Deployment: list(() => import('./deployments').then((m) => m.DeploymentComponents)),
  SwarmService: list(() => import('./swarm-services').then((m) => m.SwarmServiceComponents)),
  Stack: list(() => import('./stacks').then((m) => m.StackComponents)),
  Activity: list(() => import('./activities').then((m) => m.ActivityComponents)),
  AlertRule: list(() => import('./alerters/alert-rules').then((m) => m.AlertRuleComponents)),
  Alert: list(() => import('./alerters/alert-events').then((m) => m.AlertEventComponents)),
  GitRepository: list(() => import('./git-repos').then((m) => m.GitRepoComponents)),
  Access: list(() => import('./access').then((m) => m.AccessComponents)),
  User: list(() => import('./access').then((m) => m.AccessComponents)),
  Team: list(() => import('./access').then((m) => m.AccessComponents)),
  Role: list(() => import('./access').then((m) => m.AccessComponents)),
  ServiceAccount: list(() => import('./access').then((m) => m.AccessComponents)),
  Binding: list(() => import('./bindings').then((m) => m.BindingComponents)),
  Tag: list(() => import('./tags').then((m) => m.TagComponents)),
  OidcProvider: list(() => import('./oidc-providers').then((m) => m.OidcProviderComponents)),
  Automation: list(() => import('./automation-actions').then((m) => m.AutomationActionComponents)),
  AutomationAction: list(() => import('./automation-actions').then((m) => m.AutomationActionComponents)),
  Build: list(() => import('./builds').then((m) => m.BuildComponents)),
  BuildAgentPool: list(() => import('./build-pools').then((m) => m.BuildPoolComponents)),
  BackupRepository: list(() => import('./backup-repositories').then((m) => m.BackupRepositoryComponents)),
  BackupPolicy: list(() => import('./backup-policies').then((m) => m.BackupPolicyComponents)),
};

export const ResourceFormPages: Partial<Record<ResourceType, ReturnType<typeof form>>> = {
  Volume: form(() => import('./docker-resources/volumes').then((m) => m.VolumeFormComponents)),
  Network: form(() => import('./docker-resources/networks').then((m) => m.NetworkFormComponents)),
  Secret: form(() => import('./swarm-resources/secrets/form').then((m) => m.SecretFormComponents)),
  Config: form(() => import('./swarm-resources/configs/form').then((m) => m.ConfigFormComponents)),
  Platform: form(() => import('./platforms/forms').then((m) => m.PlatformFormComponents)),
  Registry: form(() => import('./registries/form').then((m) => m.RegistryFormComponents)),
  Deployment: form(() => import('./deployments/form').then((m) => m.DeploymentFormComponents)),
  SwarmService: form(() => import('./swarm-services/form').then((m) => m.SwarmServiceFormComponents)),
  AlertRule: form(() => import('./alerters/alert-rules/form').then((m) => m.AlertRuleFormComponents)),
  Stack: form(() => import('./stacks/form').then((m) => m.StackFormComponents)),
  GitRepository: form(() => import('./git-repos/form').then((m) => m.GitRepoFormComponents)),
  User: form(() => import('./access/users/form').then((m) => m.UserFormComponents)),
  Team: form(() => import('./access/teams/form').then((m) => m.TeamFormComponents)),
  ServiceAccount: form(() => import('./access/service-accounts/form').then((m) => m.ServiceAccountFormComponents)),
  OidcProvider: form(() => import('./oidc-providers/form').then((m) => m.OidcProviderFormComponents)),
  AutomationAction: form(() => import('./automation-actions/form').then((m) => m.AutomationActionFormComponents)),
  Build: form(() => import('./builds/form').then((m) => m.BuildFormComponents)),
  BuildAgentPool: form(() => import('./build-pools/form').then((m) => m.BuildPoolFormComponents)),
  BackupRepository: form(() => import('./backup-repositories/form').then((m) => m.BackupRepositoryFormComponents)),
  BackupPolicy: form(() => import('./backup-policies/form').then((m) => m.BackupPolicyFormComponents)),
};

export const DockerResourceInfoPages: Record<DockerResourceType, ReturnType<typeof detail>> = {
  Image: detail(() => import('./docker-resources/images/image-info').then((m) => m.ImageInfoComponents)),
  Volume: detail(() => import('./docker-resources/volumes/volume-info').then((m) => m.VolumeInfoComponents)),
  Network: detail(() => import('./docker-resources/networks/network-info').then((m) => m.NetworkInfoComponents)),
  Container: detail(() =>
    import('./docker-resources/containers/container-info').then((m) => m.ContainerInfoComponents),
  ),
};

export const SwarmResourceInfoPages: Record<SwarmResourceType, ReturnType<typeof detail>> = {
  Node: detail(() => import('./swarm-resources/nodes/node-info').then((m) => m.NodeInfoComponents)),
  Service: detail(() => import('./swarm-resources/services/service-info').then((m) => m.ServiceInfoComponents)),
  Task: detail(() => import('./swarm-resources/tasks/task-info').then((m) => m.TaskInfoComponents)),
  Secret: detail(() => import('./swarm-resources/secrets/secret-info').then((m) => m.SecretInfoComponents)),
  Config: detail(() => import('./swarm-resources/configs/config-info').then((m) => m.ConfigInfoComponents)),
};
