using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Domain.Entities.Licensing;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Licensing;

public sealed class LicenseQuotaTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private const string BusinessLicense = "test.header.signature";
    private const string FutureBusinessLicense = "future.header.signature";

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<ILicenseVerifier>(new TestLicenseVerifier());
    }

    [Fact]
    public async Task License_Endpoints_Without_Installed_License_Should_Use_Stable_Community_State()
    {
        var cancellationToken = TestContext.Current.CancellationToken;

        var licenseResponse = await Client.GetAsync("/api/v1/license", cancellationToken);
        licenseResponse.EnsureSuccessStatusCode();
        var license = await ReadJsonAsync(licenseResponse, cancellationToken);

        var requestResponse = await Client.GetAsync("/api/v1/license/request", cancellationToken);
        requestResponse.EnsureSuccessStatusCode();
        var request = await ReadJsonAsync(requestResponse, cancellationToken);

        var repeatedLicenseResponse = await Client.GetAsync("/api/v1/license", cancellationToken);
        repeatedLicenseResponse.EnsureSuccessStatusCode();
        var repeatedLicense = await ReadJsonAsync(repeatedLicenseResponse, cancellationToken);

        Assert.Equal("Community", license.GetProperty("status").GetString());
        Assert.Equal("Community", license.GetProperty("edition").GetString());
        Assert.Equal(
            license.GetProperty("instanceId").GetGuid(),
            request.GetProperty("instanceId").GetGuid());
        Assert.Equal(
            license.GetProperty("instanceId").GetGuid(),
            repeatedLicense.GetProperty("instanceId").GetGuid());
        Assert.Equal(
            CommunityLicenseLimits.Values[LicenseLimit.CustomRoles],
            GetLimit(license, LicenseLimit.CustomRoles).Maximum);
        Assert.Equal(
            CommunityLicenseLimits.Values[LicenseLimit.ActiveUsers],
            GetLimit(license, LicenseLimit.ActiveUsers).Maximum);
        Assert.False(license.TryGetProperty("rawLicense", out _));
    }

    [Fact]
    public async Task Create_Role_At_License_Boundary_Should_Succeed_And_Next_Create_Should_Return_Stable_Quota_Error()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallBusinessLicenseAsync(cancellationToken);

        var beforeResponse = await Client.GetAsync("/api/v1/license", cancellationToken);
        beforeResponse.EnsureSuccessStatusCode();
        var before = await ReadJsonAsync(beforeResponse, cancellationToken);
        Assert.Equal(new LimitState(0, 1, false), GetLimit(before, LicenseLimit.CustomRoles));

        var allowedResponse = await CreateRoleAsync("BoundaryRole", cancellationToken);
        allowedResponse.EnsureSuccessStatusCode();

        var deniedResponse = await CreateRoleAsync("OverBoundaryRole", cancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, deniedResponse.StatusCode);
        var problem = await ReadJsonAsync(deniedResponse, cancellationToken);
        var violation = Assert.Single(problem.GetProperty("violations").EnumerateArray());

        Assert.Equal("License quota exceeded.", problem.GetProperty("detail").GetString());
        Assert.Equal("Valid", problem.GetProperty("licenseStatus").GetString());
        Assert.Equal("Business", problem.GetProperty("edition").GetString());
        Assert.Equal("CustomRoles", violation.GetProperty("limit").GetString());
        Assert.Equal(1, violation.GetProperty("current").GetInt32());
        Assert.Equal(1, violation.GetProperty("requested").GetInt32());
        Assert.Equal(1, violation.GetProperty("maximum").GetInt32());

        var afterResponse = await Client.GetAsync("/api/v1/license", cancellationToken);
        afterResponse.EnsureSuccessStatusCode();
        var after = await ReadJsonAsync(afterResponse, cancellationToken);
        Assert.Equal(new LimitState(1, 1, false), GetLimit(after, LicenseLimit.CustomRoles));
    }

    [Fact]
    public async Task Installing_Future_Dated_License_Should_Keep_Community_Limits_Until_NotBefore()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var installResponse = await Client.PostAsJsonAsync(
            "/api/v1/license",
            new { license = FutureBusinessLicense },
            cancellationToken);
        installResponse.EnsureSuccessStatusCode();
        var state = await ReadJsonAsync(installResponse, cancellationToken);

        Assert.Equal("NotYetValid", state.GetProperty("status").GetString());
        Assert.Equal("Community", state.GetProperty("edition").GetString());
        Assert.Equal(new LimitState(0, 0, false), GetLimit(state, LicenseLimit.CustomRoles));

        var createResponse = await CreateRoleAsync("BlockedBeforeNotBefore", cancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, createResponse.StatusCode);
    }

    [Fact]
    public async Task Concurrent_Quota_Checks_With_One_Remaining_Slot_Should_Serialize_And_Reject_Second_Increase()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallBusinessLicenseAsync(cancellationToken);

        await using var firstScope = Services.CreateAsyncScope();
        await using var secondScope = Services.CreateAsyncScope();
        var firstUnitOfWork = firstScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var secondUnitOfWork = secondScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var quotaService = Services.GetRequiredService<ILicenseQuotaService>();
        var increase = new Dictionary<LicenseLimit, int>
        {
            [LicenseLimit.CustomRoles] = 1
        };

        var firstCheck = await quotaService.EnsureCanIncreaseAsync(
            increase,
            firstUnitOfWork,
            cancellationToken);
        Assert.True(firstCheck.IsSuccess());

        var secondCheckTask = quotaService.EnsureCanIncreaseAsync(
            increase,
            secondUnitOfWork,
            cancellationToken).AsTask();
        var earlyCompletion = await Task.WhenAny(
            secondCheckTask,
            Task.Delay(TimeSpan.FromMilliseconds(250), cancellationToken));
        Assert.NotSame(secondCheckTask, earlyCompletion);

        await firstUnitOfWork.Roles.AddAsync(
            Role.Create("ConcurrentBoundaryRole", RoleType.Custom),
            cancellationToken);
        await firstUnitOfWork.CommitAsync(cancellationToken);

        var secondCheck = await secondCheckTask.WaitAsync(TimeSpan.FromSeconds(5), cancellationToken);
        Assert.True(secondCheck.IsFailure(out var error));
        Assert.Equal("License quota exceeded.", error.Message);
        await secondUnitOfWork.RollbackAsync();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUnitOfWork = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var customRoles = (await verificationUnitOfWork.Roles.GetAllAsync(cancellationToken))
            .Count(role => role.RoleType == RoleType.Custom);
        Assert.Equal(1, customRoles);
    }

    [Fact]
    public async Task Removing_License_Should_Keep_Existing_Resources_Usable_And_Apply_Community_Limits_Immediately()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallBusinessLicenseAsync(cancellationToken);

        var createResponse = await CreateRoleAsync("PreservedRole", cancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var createdRole = await ReadJsonAsync(createResponse, cancellationToken);
        var roleId = createdRole.GetProperty("id").GetGuid();

        var removeResponse = await Client.DeleteAsync("/api/v1/license", cancellationToken);
        removeResponse.EnsureSuccessStatusCode();
        var removedState = await ReadJsonAsync(removeResponse, cancellationToken);

        Assert.Equal("Community", removedState.GetProperty("status").GetString());
        Assert.Equal(new LimitState(1, 0, true), GetLimit(removedState, LicenseLimit.CustomRoles));

        var renameResponse = await Client.PostAsJsonAsync(
            "/api/v1/roles/rename",
            new { id = roleId, name = "PreservedRoleRenamed" },
            cancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        var deniedCreateResponse = await CreateRoleAsync("BlockedAfterDowngrade", cancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, deniedCreateResponse.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = await unitOfWork.Roles.GetAsync(roleId, cancellationToken);
        Assert.Equal("PreservedRoleRenamed", role?.Name);
    }

    private async Task InstallBusinessLicenseAsync(CancellationToken cancellationToken)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/license",
            new { license = BusinessLicense },
            cancellationToken);
        response.EnsureSuccessStatusCode();
    }

    private Task<HttpResponseMessage> CreateRoleAsync(string name, CancellationToken cancellationToken)
        => Client.PostAsJsonAsync(
            "/api/v1/roles",
            new
            {
                name,
                permissions = Array.Empty<object>()
            },
            cancellationToken);

    private static async Task<JsonElement> ReadJsonAsync(
        HttpResponseMessage response,
        CancellationToken cancellationToken)
        => await response.Content.ReadFromJsonAsync<JsonElement>(cancellationToken);

    private static LimitState GetLimit(JsonElement license, LicenseLimit limit)
    {
        var item = license.GetProperty("limits")
            .EnumerateArray()
            .Single(x => string.Equals(
                x.GetProperty("limit").GetString(),
                limit.ToString(),
                StringComparison.Ordinal));

        return new LimitState(
            item.GetProperty("current").GetInt32(),
            item.GetProperty("maximum").GetInt32(),
            item.GetProperty("overQuota").GetBoolean());
    }

    private sealed record LimitState(int Current, int Maximum, bool OverQuota);

    private sealed class TestLicenseVerifier : ILicenseVerifier
    {
        public LicenseVerificationResult Verify(
            string rawLicense,
            CitadelInstanceIdentity instance,
            DateTimeOffset now)
        {
            var status = rawLicense switch
            {
                BusinessLicense => LicenseStatus.Valid,
                FutureBusinessLicense => LicenseStatus.NotYetValid,
                _ => (LicenseStatus?)null
            };
            if (status is null)
            {
                return LicenseVerificationResult.Failed(
                    LicenseStatus.Invalid,
                    "LICENSE_INVALID",
                    "License is not valid.");
            }

            var effectiveLimits = new Dictionary<LicenseLimit, int>(CommunityLicenseLimits.Values)
            {
                [LicenseLimit.CustomRoles] = 1
            };
            var payload = new LicensePayload(
                Schema: LicenseConstants.CurrentSchema,
                Product: LicenseConstants.Product,
                Issuer: LicenseConstants.Issuer,
                Audience: LicenseConstants.Audience,
                LicenseId: "license-integration-test",
                ReplacedLicenseId: null,
                Customer: new LicenseCustomer("customer-integration-test", "Integration Test"),
                Edition: LicenseConstants.EditionBusiness,
                InstanceId: instance.InstanceId,
                IssuedAt: now.AddDays(-1),
                NotBefore: status == LicenseStatus.NotYetValid ? now.AddDays(1) : now.AddDays(-1),
                ExpiresAt: now.AddDays(30),
                GraceUntil: now.AddDays(44),
                Limits: new Dictionary<string, int>
                {
                    [LicenseLimitKeys.CustomRoles] = 1
                });
            var license = new VerifiedLicense(
                RawLicense: rawLicense,
                Fingerprint: "integration-test-fingerprint",
                KeyId: "integration-test-key",
                Payload: payload,
                Status: status.Value,
                EffectiveLimits: effectiveLimits,
                Warnings: []);

            return LicenseVerificationResult.Succeeded(license);
        }
    }
}
