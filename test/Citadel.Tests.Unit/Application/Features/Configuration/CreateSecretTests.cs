using Application.Features.Configuration.Commands;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Moq;

namespace Tests.Unit.Application.Features.Configuration;

public sealed class CreateSecretTests
{
    [Fact]
    public async Task CreateInternalSecret_Should_Reject_Duplicate_Name()
    {
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets
            .Setup(x => x.ExistsByNameAsync("API_KEY", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        var unitOfWork = CreateUnitOfWork(secrets.Object);
        var handler = new CreateInternalSecretHandler(unitOfWork.Object, new TestSecretValueProtector());

        var result = await handler.Handle(new CreateInternalSecret("API_KEY", "secret"), TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Name already exists", error.Message);
        secrets.Verify(x => x.AddAsync(It.IsAny<SecretDefinition>(), It.IsAny<InternalSecretValue>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task CreateExternalSecret_Should_Reject_Duplicate_Name_Before_Provider_Lookup()
    {
        var secrets = new Mock<ISecretDefinitionRepository>();
        secrets
            .Setup(x => x.ExistsByNameAsync("API_KEY", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        var providers = new Mock<ISecretProviderRepository>();
        var unitOfWork = CreateUnitOfWork(secrets.Object, providers.Object);
        var handler = new CreateExternalSecretHandler(unitOfWork.Object);

        var result = await handler.Handle(
            new CreateExternalSecret("API_KEY", Guid.CreateVersion7(), "apps/api/prod", "api_key", null),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Name already exists", error.Message);
        providers.Verify(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        secrets.Verify(x => x.AddAsync(It.IsAny<SecretDefinition>(), null, It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        ISecretDefinitionRepository secrets,
        ISecretProviderRepository? providers = null)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.SecretDefinitions).Returns(secrets);
        if (providers is not null)
            unitOfWork.Setup(x => x.SecretProviders).Returns(providers);
        return unitOfWork;
    }

    private sealed class TestSecretValueProtector : ISecretValueProtector
    {
        public string Protect(string value) => $"protected:{value}";

        public string Unprotect(string protectedValue) => protectedValue;
    }
}
