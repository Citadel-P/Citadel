using static Agent.Server.Containers.Containers;
using static Agent.Server.GPlatform.gPlatform;

namespace Infrastructure.Services.Abstractions;

/// <summary>
/// Factory to create gRPC clients (natif grpc factory does not support address change in runtime see https://github.com/grpc/grpc-dotnet/issues/1641)
/// </summary>
public interface IGrpcClientFactory
{
    gPlatformClient GetPlatformClient(string address);
    ContainersClient GetContainerClient(string address);
}
