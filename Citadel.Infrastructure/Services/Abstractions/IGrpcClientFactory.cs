using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Images.V1.ImageService;
using static Citadel.Agent.Networks.V1.NetworkService;
using static Citadel.Agent.Platforms.V1.PlatformService;
using static Citadel.Agent.Volumes.V1.VolumeService;

namespace Infrastructure.Services.Abstractions;

/// <summary>
/// Factory to create gRPC clients (natif grpc factory does not support address change in runtime see https://github.com/grpc/grpc-dotnet/issues/1641)
/// </summary>
public interface IGrpcClientFactory
{
    PlatformServiceClient GetPlatformClient(string address);
    ContainerServiceClient GetContainerClient(string address);
    ImageServiceClient GetImageClient(string address);
    NetworkServiceClient GetNetworkClient(string address);
    VolumeServiceClient GetVolumeClient(string address);
}
