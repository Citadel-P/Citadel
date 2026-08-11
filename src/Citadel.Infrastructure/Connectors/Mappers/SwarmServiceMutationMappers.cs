using Domain.Contracts.Resources.Swarm;
using Domain.Entities.SwarmServices;
using Hosting.DockerClient.Models.Swarm;

namespace Infrastructure.Connectors.Mappers;

internal static class SwarmServiceMutationMappers
{
    internal static CreateSwarmServiceCommand Map(CreateManagedSwarmServiceCommand command) =>
        new(
            command.OperationId,
            command.DockerName,
            Map(command.Spec, command.ResolvedImage),
            command.Labels,
            command.RegistryAuth);

    internal static UpdateSwarmServiceCommand Map(UpdateManagedSwarmServiceCommand command) =>
        new(
            command.OperationId,
            command.ServiceId,
            command.VersionIndex,
            Map(command.Spec, command.ResolvedImage, command.ForceUpdate),
            command.Labels,
            command.RegistryAuth);

    internal static DeleteSwarmServiceCommand Map(DeleteManagedSwarmServiceCommand command) =>
        new(command.OperationId, command.ServiceId);

    internal static ManagedSwarmServiceMutationResult Map(SwarmServiceMutationResult result) =>
        new(result.ServiceId, result.Warnings);

    internal static Hosting.DockerClient.Models.Swarm.CreateSystemSwarmServiceCommand Map(
        Domain.Contracts.Resources.Swarm.CreateSystemSwarmServiceCommand command) =>
        new(command.OperationId, command.DockerName, Map(command.Spec), command.Labels, command.ContainerLabels);

    internal static Hosting.DockerClient.Models.Swarm.UpdateSystemSwarmServiceCommand Map(
        Domain.Contracts.Resources.Swarm.UpdateSystemSwarmServiceCommand command) =>
        new(command.OperationId, command.ServiceId, command.VersionIndex, Map(command.Spec), command.Labels, command.ContainerLabels);

    private static Hosting.DockerClient.Models.Swarm.SystemSwarmServiceSpec Map(
        Domain.Contracts.Resources.Swarm.SystemSwarmServiceSpec spec) =>
        new(
            spec.Image,
            spec.Environment,
            spec.ManagerNodeId,
            spec.StateVolumeName,
            spec.BootstrapSecretId,
            spec.BootstrapSecretName,
            spec.CaConfigId,
            spec.CaConfigName,
            spec.LimitNanoCpus,
            spec.LimitMemoryBytes,
            spec.PidsLimit,
            spec.StopGracePeriodNanoseconds,
            spec.SupportedArchitectures);

    private static SwarmServiceMutationSpec Map(SwarmServiceSpec spec, string image, int forceUpdate = 0) =>
        new(
            image,
            spec.SchedulingMode.ToString(),
            spec.Replicas,
            spec.Command,
            spec.Arguments,
            spec.Environment,
            spec.User,
            spec.WorkingDirectory,
            spec.HealthCheck is null ? null : new SwarmHealthCheckSpec(
                spec.HealthCheck.Test,
                spec.HealthCheck.IntervalNanoseconds,
                spec.HealthCheck.TimeoutNanoseconds,
                spec.HealthCheck.Retries,
                spec.HealthCheck.StartPeriodNanoseconds),
            spec.StopGracePeriodNanoseconds,
            spec.Ports.Select(static port => new SwarmPortSpec(
                port.TargetPort,
                port.PublishedPort,
                port.Protocol,
                port.PublishMode.ToString())).ToArray(),
            spec.NetworkIds,
            spec.Mounts.Select(static mount => new SwarmMountSpec(
                mount.Kind.ToString(), mount.Source, mount.Target, mount.ReadOnly)).ToArray(),
            spec.Secrets.Select(static secret => new SwarmSecretReferenceSpec(
                secret.SecretId, secret.SecretName, secret.TargetName)).ToArray(),
            spec.Configs.Select(static config => new SwarmConfigReferenceSpec(
                config.ConfigId, config.ConfigName, config.TargetName)).ToArray(),
            spec.Resources is null ? null : new SwarmResourceSpec(
                spec.Resources.LimitNanoCpus,
                spec.Resources.LimitMemoryBytes,
                spec.Resources.ReservationNanoCpus,
                spec.Resources.ReservationMemoryBytes),
            spec.PlacementConstraints,
            spec.RestartPolicy is null ? null : new SwarmRestartPolicySpec(
                spec.RestartPolicy.Condition.ToString(),
                spec.RestartPolicy.DelayNanoseconds,
                spec.RestartPolicy.MaximumAttempts,
                spec.RestartPolicy.WindowNanoseconds),
            spec.UpdatePolicy is null ? null : new SwarmUpdatePolicySpec(
                spec.UpdatePolicy.Parallelism,
                spec.UpdatePolicy.DelayNanoseconds,
                spec.UpdatePolicy.Order.ToString(),
                spec.UpdatePolicy.FailureAction.ToString()),
            forceUpdate);
}
