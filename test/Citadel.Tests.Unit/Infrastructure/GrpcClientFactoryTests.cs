using Infrastructure.Repositories;

namespace Tests.Unit.Infrastructure;

public sealed class GrpcClientFactoryTests
{
    [Fact]
    public void Evict_RemovesChannelAndTypedClientsForAddress()
    {
        using var factory = new GrpcClientFactory();
        const string address = "http://127.0.0.1:5001";

        factory.GetPlatformClient(address);
        factory.GetContainerClient(address);

        Assert.Equal(1, factory.ChannelCount);
        Assert.Equal(2, factory.ClientCount);

        factory.Evict(address);

        Assert.Equal(0, factory.ChannelCount);
        Assert.Equal(0, factory.ClientCount);
    }
}
