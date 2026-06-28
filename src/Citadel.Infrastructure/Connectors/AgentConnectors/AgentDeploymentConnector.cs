using Citadel.Deployments.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentDeploymentConnector(IGrpcClientFactory clientFactory) : IDeploymentConnector
{
    public async Task<Result<ApplyDeploymentResult>> ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken)
    {
        try
        {
            var deploymentClient = clientFactory.GetDeploymentClient(applyDeployment.PlatformAddress);
            var request = new ApplyDeploymentRequest() 
            {
                ImageId = applyDeployment.ImageId ?? "",
                Name = applyDeployment.Name ?? "",
                Spec = new DeploymentSpec
                {
                    LifeCycleSpec = new LifeCycleSpec
                    {
                        RestartPolicy = applyDeployment.Spec.LifeCycleSpec?.RestartPolicy.MapToAgent() ?? ContainerRestartPolicy.No,
                        StopTimeout = applyDeployment.Spec.LifeCycleSpec?.StopTimeout ?? 10,
                        StopSignal = applyDeployment.Spec.LifeCycleSpec?.StopSignal.MapToAgent() ?? StopSignal.Sigterm
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
            var result = await deploymentClient.ApplyAsync(request, cancellationToken: cancellationToken);
            return result.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<ApplyDeploymentResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
