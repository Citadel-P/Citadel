using Citadel.Swarm.V1;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.SwarmServices;

namespace Infrastructure.Connectors.Mappers;

internal static class SwarmServiceTransportMappers
{
    internal static CreateManagedSwarmServiceRequest Map(CreateManagedSwarmServiceCommand command)
    {
        var request = new CreateManagedSwarmServiceRequest
        {
            OperationId = command.OperationId.ToString("D"),
            DockerName = command.DockerName,
            Spec = Map(command.Spec, command.ResolvedImage),
            RegistryAuth = command.RegistryAuth ?? string.Empty
        };
        foreach (var (key, value) in command.Labels)
            request.Labels[key] = value;
        return request;
    }

    internal static UpdateManagedSwarmServiceRequest Map(UpdateManagedSwarmServiceCommand command)
    {
        var request = new UpdateManagedSwarmServiceRequest
        {
            OperationId = command.OperationId.ToString("D"),
            ServiceId = command.ServiceId,
            VersionIndex = checked((ulong)command.VersionIndex),
            Spec = Map(command.Spec, command.ResolvedImage, command.ForceUpdate),
            RegistryAuth = command.RegistryAuth ?? string.Empty
        };
        foreach (var (key, value) in command.Labels)
            request.Labels[key] = value;
        return request;
    }

    internal static DeleteManagedSwarmServiceRequest Map(DeleteManagedSwarmServiceCommand command) =>
        new() { OperationId = command.OperationId.ToString("D"), ServiceId = command.ServiceId };

    internal static ManagedSwarmServiceMutationResult Map(SwarmServiceMutationResponse response) =>
        new(string.IsNullOrEmpty(response.ServiceId) ? null : response.ServiceId, response.Warnings.ToArray());

    private static SwarmServiceMutationSpecMessage Map(
        SwarmServiceSpec spec,
        string image,
        int forceUpdate = 0)
    {
        var message = new SwarmServiceMutationSpecMessage
        {
            Image = image,
            SchedulingMode = spec.SchedulingMode.ToString(),
            User = spec.User ?? string.Empty,
            WorkingDirectory = spec.WorkingDirectory ?? string.Empty,
            ForceUpdate = forceUpdate
        };
        if (spec.Replicas is not null) message.Replicas = spec.Replicas.Value;
        if (spec.StopGracePeriodNanoseconds is not null) message.StopGracePeriodNanoseconds = spec.StopGracePeriodNanoseconds.Value;
        message.Command.Add(spec.Command);
        message.Arguments.Add(spec.Arguments);
        message.Environment.Add(spec.Environment);
        message.NetworkIds.Add(spec.NetworkIds);
        message.PlacementConstraints.Add(spec.PlacementConstraints);

        if (spec.HealthCheck is not null)
        {
            message.HealthCheck = new SwarmHealthCheckSpecMessage();
            message.HealthCheck.Test.Add(spec.HealthCheck.Test);
            if (spec.HealthCheck.IntervalNanoseconds is not null) message.HealthCheck.IntervalNanoseconds = spec.HealthCheck.IntervalNanoseconds.Value;
            if (spec.HealthCheck.TimeoutNanoseconds is not null) message.HealthCheck.TimeoutNanoseconds = spec.HealthCheck.TimeoutNanoseconds.Value;
            if (spec.HealthCheck.Retries is not null) message.HealthCheck.Retries = spec.HealthCheck.Retries.Value;
            if (spec.HealthCheck.StartPeriodNanoseconds is not null) message.HealthCheck.StartPeriodNanoseconds = spec.HealthCheck.StartPeriodNanoseconds.Value;
        }

        foreach (var port in spec.Ports)
        {
            var value = new SwarmPortSpecMessage
            {
                TargetPort = port.TargetPort,
                Protocol = port.Protocol,
                PublishMode = port.PublishMode.ToString()
            };
            if (port.PublishedPort is not null) value.PublishedPort = port.PublishedPort.Value;
            message.Ports.Add(value);
        }

        message.Mounts.Add(spec.Mounts.Select(static mount => new SwarmMountSpecMessage
        {
            Kind = mount.Kind.ToString(), Source = mount.Source, Target = mount.Target, ReadOnly = mount.ReadOnly
        }));
        message.Secrets.Add(spec.Secrets.Select(static secret => new SwarmSecretReferenceSpecMessage
        {
            Id = secret.SecretId, Name = secret.SecretName, TargetName = secret.TargetName
        }));
        message.Configs.Add(spec.Configs.Select(static config => new SwarmConfigReferenceSpecMessage
        {
            Id = config.ConfigId, Name = config.ConfigName, TargetName = config.TargetName
        }));

        if (spec.Resources is not null)
        {
            message.Resources = new SwarmResourceSpecMessage();
            if (spec.Resources.LimitNanoCpus is not null) message.Resources.LimitNanoCpus = spec.Resources.LimitNanoCpus.Value;
            if (spec.Resources.LimitMemoryBytes is not null) message.Resources.LimitMemoryBytes = spec.Resources.LimitMemoryBytes.Value;
            if (spec.Resources.ReservationNanoCpus is not null) message.Resources.ReservationNanoCpus = spec.Resources.ReservationNanoCpus.Value;
            if (spec.Resources.ReservationMemoryBytes is not null) message.Resources.ReservationMemoryBytes = spec.Resources.ReservationMemoryBytes.Value;
        }

        if (spec.RestartPolicy is not null)
        {
            message.RestartPolicy = new SwarmRestartPolicySpecMessage { Condition = spec.RestartPolicy.Condition.ToString() };
            if (spec.RestartPolicy.DelayNanoseconds is not null) message.RestartPolicy.DelayNanoseconds = spec.RestartPolicy.DelayNanoseconds.Value;
            if (spec.RestartPolicy.MaximumAttempts is not null) message.RestartPolicy.MaximumAttempts = spec.RestartPolicy.MaximumAttempts.Value;
            if (spec.RestartPolicy.WindowNanoseconds is not null) message.RestartPolicy.WindowNanoseconds = spec.RestartPolicy.WindowNanoseconds.Value;
        }

        if (spec.UpdatePolicy is not null)
        {
            message.UpdatePolicy = new SwarmUpdatePolicySpecMessage
            {
                Parallelism = spec.UpdatePolicy.Parallelism,
                Order = spec.UpdatePolicy.Order.ToString(),
                FailureAction = spec.UpdatePolicy.FailureAction.ToString()
            };
            if (spec.UpdatePolicy.DelayNanoseconds is not null) message.UpdatePolicy.DelayNanoseconds = spec.UpdatePolicy.DelayNanoseconds.Value;
        }

        return message;
    }
}
