using Application;
using Application.Services;
using Application.Services.Backups;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Unit.Application;

public sealed class ApplicationModuleTests
{
    [Fact]
    public void RegisterApplicationModule_RegistersNetworkServiceAsScoped()
    {
        ServiceCollection services = new();

        services.RegisterApplicationModule();

        var descriptor = Assert.Single(
            services.Where(service => service.ServiceType == typeof(INetworkService)));
        Assert.Equal(ServiceLifetime.Scoped, descriptor.Lifetime);
        Assert.Equal(typeof(NetworkService), descriptor.ImplementationType);
    }

    [Fact]
    public void RegisterApplicationModule_RegistersBackupRunExecutionServiceAsScoped()
    {
        ServiceCollection services = new();

        services.RegisterApplicationModule();

        var descriptor = Assert.Single(
            services.Where(service => service.ServiceType == typeof(IBackupRunExecutionService)));
        Assert.Equal(ServiceLifetime.Scoped, descriptor.Lifetime);
        Assert.Equal(typeof(BackupRunExecutionService), descriptor.ImplementationType);
    }
}
