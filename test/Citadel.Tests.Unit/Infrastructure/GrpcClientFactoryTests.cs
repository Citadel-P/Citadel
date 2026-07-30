using Domain.Configs;
using Hosting.Common.Security;
using Infrastructure.Repositories;
using Microsoft.Extensions.Options;

namespace Tests.Unit.Infrastructure;

public sealed class GrpcClientFactoryTests
{
    [Fact]
    public void Evict_RemovesChannelAndTypedClientsForAddress()
    {
        var transportOptions = Options.Create(new AgentTransportOptions
        {
            AllowInsecure = true
        });
        using var certificateTrust = CertificateTrust.Load(null);
        using var factory = new GrpcClientFactory(
            transportOptions,
            certificateTrust);
        const string address = "http://127.0.0.1:5001";

        factory.GetPlatformClient(address);
        factory.GetContainerClient(address);

        Assert.Equal(1, factory.ChannelCount);
        Assert.Equal(2, factory.ClientCount);

        factory.Evict(address);

        Assert.Equal(0, factory.ChannelCount);
        Assert.Equal(0, factory.ClientCount);
    }

    [Fact]
    public void GetPlatformClient_RejectsHttpWhenInsecureTransportIsDisabled()
    {
        var transportOptions = Options.Create(
            new AgentTransportOptions { AllowInsecure = false });
        using var certificateTrust = CertificateTrust.Load(null);
        using var factory = new GrpcClientFactory(
            transportOptions,
            certificateTrust);

        var exception = Assert.Throws<InvalidOperationException>(
            () => factory.GetPlatformClient("http://127.0.0.1:5001"));

        Assert.Contains("must use HTTPS", exception.Message);
        Assert.Equal(0, factory.ChannelCount);
    }

    [Fact]
    public void GetPlatformClient_RejectsAddressWithoutScheme()
    {
        var transportOptions = Options.Create(
            new AgentTransportOptions { AllowInsecure = true });
        using var certificateTrust = CertificateTrust.Load(null);
        using var factory = new GrpcClientFactory(
            transportOptions,
            certificateTrust);

        Assert.Throws<ArgumentException>(
            () => factory.GetPlatformClient("127.0.0.1:5001"));
        Assert.Equal(0, factory.ChannelCount);
    }

    [Fact]
    public void GetPlatformClient_ReusesCachedClient()
    {
        var transportOptions = Options.Create(
            new AgentTransportOptions { AllowInsecure = true });
        using var certificateTrust = CertificateTrust.Load(null);
        using var factory = new GrpcClientFactory(
            transportOptions,
            certificateTrust);
        const string address = "http://127.0.0.1:5001";

        var first = factory.GetPlatformClient(address);
        var second = factory.GetPlatformClient(address);

        Assert.Same(first, second);
        Assert.Equal(1, factory.ChannelCount);
        Assert.Equal(1, factory.ClientCount);
    }
}
