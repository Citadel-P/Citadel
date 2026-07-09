using Citadel.Deployments.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeDeploymentConnector(IEdgeAgentCommandRouter commandRouter) : IDeploymentConnector
{
    public async Task<Result<ApplyDeploymentResult>> ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(applyDeployment.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<ApplyDeploymentResult>(addressError!);
        }

        var request = new ApplyDeploymentRequest
        {
            ImageId = applyDeployment.ImageId ?? string.Empty,
            Name = applyDeployment.Name ?? string.Empty,
            Spec = new DeploymentSpec
            {
                LifeCycleSpec = new LifeCycleSpec
                {
                    RestartPolicy = applyDeployment.Spec.LifeCycleSpec?.RestartPolicy.MapToAgent() ?? Citadel.Deployments.V1.ContainerRestartPolicy.No,
                    StopTimeout = applyDeployment.Spec.LifeCycleSpec?.StopTimeout ?? 10,
                    StopSignal = applyDeployment.Spec.LifeCycleSpec?.StopSignal.MapToAgent() ?? Citadel.Deployments.V1.StopSignal.Sigterm
                },
                ResourceSpec = new ResourceSpec
                {
                    NanoCpus = applyDeployment.Spec.ResourceSpec?.NanoCpus ?? 0,
                    MemoryLimit = applyDeployment.Spec.ResourceSpec?.MemoryLimit ?? 0
                },
                Labels = { applyDeployment.Spec.Labels ?? [] },
                Networks = { applyDeployment.Spec.Networks ?? [] },
                Command = { applyDeployment.Spec.Command ?? [] },
                EnvVars = { applyDeployment.EnvironmentVariables ?? [] },
                Ports = { applyDeployment.Spec.Ports ?? [] },
                Volumes = { applyDeployment.Spec.Volumes ?? [] }
            }
        };

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.DeploymentApply,
            request.ToByteArray(),
            TimeSpan.FromMinutes(5),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? ApplyDeploymentResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<ApplyDeploymentResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.DeploymentApply, response));
    }
}
