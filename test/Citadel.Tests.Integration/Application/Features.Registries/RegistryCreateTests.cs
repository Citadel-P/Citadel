using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IRegistryConnectorStrategy> registryConnectorMock = new();
    private readonly Mock<IRegistryConnectorResolver> registryConnectorResolverMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services
            .AddScoped(_ => registryConnectorMock.Object)
            .AddScoped(_ => registryConnectorResolverMock.Object);
    }

    [Fact]
    public async Task Create_DockerHubRegistry_ReturnsSuccess()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
           .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((true, null));

        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "status": "Active",
          "configuration": {
            "$type": "DockerHub",
            "userName": "dummy-user",
            "PAT": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(registries, r => r.Name == "R-NEW");
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_CustomRegistry_ReturnsSuccess()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
           .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((true, null));

        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "description": "A custom registry",
          "status": "Disabled",
          "configuration": {
            "$type": "Custom",
            "authEnabled": true,
            "userName": "dummy-user",
            "password": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(registries, r => r.Name == "R-NEW");
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitHubRegistry_ReturnsSuccess()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
           .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((true, null));

        var createJson = """
        {
          "name": "R-NEW",
          "status": "Active",
          "description": "A GitHub registry",
          "registryHost": "ghcr.io",
          "configuration": {
            "$type": "GitHub",
            "ghcrAuthEnabled": true,
            "nameSpace": "my-org",
            "PAT": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(registries, r => r.Name == "R-NEW");
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Registry_With_Empty_Name_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "",
          "registryHost": "registry123:9999",
          "configuration": {
            "$type": "DockerHub",
            "userName": "dummy-user",
            "PAT": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    //[Fact]
    //public async Task Create_Registry_With_Invalid_$type_Returns_BadRequest()
    //{
    //    var createJson = """
    //    {
    //      "name": "R-NEW",
    //      "registryHost": "registry123:9999",
    //      "configuration": {
    //        "$type": "Degraded$type",
    //        "userName": "dummy-user",
    //        "PAT": "dummy-pat123"
    //      }
    //    }
    //    """;
    //    var content = new StringContent(createJson, Encoding.UTF8, "application/json");

    //    var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

    //    var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
    //    Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    //    await VerifyJson(responseBody);
    //}

    [Fact]
    public async Task Create_Registry_With_Invalid_Configuration_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "configuration": {
            "$type": "DockerHub",
            "userName": "",
            "PAT": "short"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Registry_With_Duplicate_Name_Returns_Conflict()
    {
        // Seed a registry with the same name
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Registries.AddAsync(new Registry(
                name: "R-NEW",
                status: RegistryStatus.Active,
                registryHost: "https://existing.url",
                createdByActorId: Constants.SystemId,
                configuration: new DockerHubRegistry("user", "dummy-pat1234")
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((true, null));

        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "configuration": {
            "$type": "DockerHub",
            "userName": "dummy-user",
            "PAT": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Registry_With_UnsupportedType_Returns_BadRequest()
    {
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns((IRegistryConnectorStrategy?)null);

        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "configuration": {
            "$type": "AWS",
            "accessKey": "accesskey1234",
            "secretAccessKey": "secretkey1234",
            "region": "us-west-1"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Registry_When_Connector_Fails_Returns_BadRequest()
    {
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((false, "Connection failed"));

        var createJson = """
        {
          "name": "R-NEW",
          "registryHost": "registry123:9999",
          "configuration": {
            "$type": "DockerHub",
            "userName": "dummy-user",
            "PAT": "dummy-pat123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

}
