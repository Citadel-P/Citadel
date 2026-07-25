using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Application.Services.Licensing;
using Domain;
using Domain.Entities.Licensing;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Licensing;

public sealed class LicenseEntitlementTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private const string TeamLicense = "team.header.signature";
    private const string FutureTeamLicense = "future.header.signature";
    private const string UnrelatedTeamLicense = "unrelated.header.signature";
    private const string ReplacementTeamLicense = "replacement.header.signature";
    private const string LegacyBusinessLicense = "legacy.header.signature";

    protected override bool UseRealLicenseEntitlements => true;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<ILicenseVerifier>(new TestLicenseVerifier());
    }

    [Fact]
    public async Task License_Endpoints_Without_Installed_License_Should_Return_Stable_Community_State()
    {
        var cancellationToken = TestContext.Current.CancellationToken;

        var licenseResponse = await Client.GetAsync("/api/v1/license", cancellationToken);
        licenseResponse.EnsureSuccessStatusCode();
        var license = await ReadJsonAsync(licenseResponse, cancellationToken);

        var entitlementsResponse = await Client.GetAsync("/api/v1/license/entitlements", cancellationToken);
        entitlementsResponse.EnsureSuccessStatusCode();
        var entitlements = await ReadJsonAsync(entitlementsResponse, cancellationToken);

        var requestResponse = await Client.GetAsync("/api/v1/license/request", cancellationToken);
        requestResponse.EnsureSuccessStatusCode();
        var request = await ReadJsonAsync(requestResponse, cancellationToken);

        Assert.Equal("Community", license.GetProperty("status").GetString());
        Assert.Equal("Community", license.GetProperty("effectiveEdition").GetString());
        Assert.False(license.TryGetProperty("licensedEdition", out _));
        Assert.Equal(
            license.GetProperty("instanceId").GetGuid(),
            request.GetProperty("instanceId").GetGuid());
        Assert.All(
            license.GetProperty("capabilities").EnumerateArray(),
            capability => Assert.False(capability.GetProperty("enabled").GetBoolean()));
        Assert.Equal("Community", entitlements.GetProperty("effectiveEdition").GetString());
        Assert.False(license.TryGetProperty("rawLicense", out _));
    }

    [Fact]
    public async Task Entitlements_Endpoint_Should_Not_Require_License_Administration_Permission()
    {
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(Guid.CreateVersion7(), Guid.CreateVersion7()));

        var response = await Client.GetAsync(
            "/api/v1/license/entitlements",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task Community_Should_Reject_Custom_Role_Creation_With_Stable_Capability_Error()
    {
        var response = await CreateRoleAsync(
            "CommunityBlockedRole",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
        var problem = await ReadJsonAsync(response, TestContext.Current.CancellationToken);
        Assert.Equal(
            "https://citadel.local/problems/license-capability-required",
            problem.GetProperty("type").GetString());
        Assert.Equal("CustomAccessControl", problem.GetProperty("capability").GetString());
        Assert.Equal("Community", problem.GetProperty("licenseStatus").GetString());
        Assert.Equal("Community", problem.GetProperty("effectiveEdition").GetString());
    }

    [Fact]
    public async Task Team_Capability_Should_Enable_Custom_Roles_Without_Row_Quotas()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var communityResponse = await Client.GetAsync("/api/v1/license/entitlements", cancellationToken);
        communityResponse.EnsureSuccessStatusCode();
        await InstallLicenseAsync(TeamLicense, cancellationToken);

        var first = await CreateRoleAsync("TeamRoleOne", cancellationToken);
        first.EnsureSuccessStatusCode();
        var second = await CreateRoleAsync("TeamRoleTwo", cancellationToken);
        second.EnsureSuccessStatusCode();

        var entitlementsResponse = await Client.GetAsync("/api/v1/license/entitlements", cancellationToken);
        entitlementsResponse.EnsureSuccessStatusCode();
        var entitlements = await ReadJsonAsync(entitlementsResponse, cancellationToken);

        Assert.Equal("Team", entitlements.GetProperty("effectiveEdition").GetString());
        Assert.True(GetCapability(entitlements, LicenseCapability.CustomAccessControl));
    }

    [Fact]
    public async Task Future_Dated_License_Should_Keep_Community_Capabilities_Until_NotBefore()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var response = await InstallLicenseAsync(FutureTeamLicense, cancellationToken);
        var state = await ReadJsonAsync(response, cancellationToken);

        Assert.Equal("NotYetValid", state.GetProperty("status").GetString());
        Assert.Equal("Community", state.GetProperty("effectiveEdition").GetString());
        Assert.Equal("Team", state.GetProperty("licensedEdition").GetString());
        Assert.False(GetCapability(state, LicenseCapability.CustomAccessControl));

        var createResponse = await CreateRoleAsync("BlockedBeforeNotBefore", cancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, createResponse.StatusCode);
    }

    [Fact]
    public async Task License_Replacement_Should_Require_Exact_Replaced_License_Id()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallLicenseAsync(TeamLicense, cancellationToken);

        var mismatch = await Client.PostAsJsonAsync(
            "/api/v1/license",
            new { license = UnrelatedTeamLicense },
            cancellationToken);
        Assert.Equal(HttpStatusCode.Conflict, mismatch.StatusCode);
        var mismatchProblem = await ReadJsonAsync(mismatch, cancellationToken);
        Assert.Equal(
            "https://citadel.local/problems/license-replacement-mismatch",
            mismatchProblem.GetProperty("type").GetString());

        var replacement = await InstallLicenseAsync(ReplacementTeamLicense, cancellationToken);
        var state = await ReadJsonAsync(replacement, cancellationToken);
        Assert.Equal("license-replacement", state.GetProperty("licenseId").GetString());
        Assert.Equal("license-team", state.GetProperty("replacedLicenseId").GetString());
    }

    [Fact]
    public async Task Removing_License_Should_Preserve_Existing_Role_But_Block_New_Custom_Roles()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallLicenseAsync(TeamLicense, cancellationToken);

        var createResponse = await CreateRoleAsync("PreservedRole", cancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var createdRole = await ReadJsonAsync(createResponse, cancellationToken);
        var roleId = createdRole.GetProperty("id").GetGuid();

        var removeResponse = await Client.DeleteAsync("/api/v1/license", cancellationToken);
        removeResponse.EnsureSuccessStatusCode();
        var removedState = await ReadJsonAsync(removeResponse, cancellationToken);
        Assert.Equal("Community", removedState.GetProperty("effectiveEdition").GetString());

        var renameResponse = await Client.PostAsJsonAsync(
            "/api/v1/roles/rename",
            new { id = roleId, name = "PreservedRoleRenamed" },
            cancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        var deniedCreateResponse = await CreateRoleAsync("BlockedAfterDowngrade", cancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, deniedCreateResponse.StatusCode);
    }

    [Fact]
    public async Task Community_Should_Not_Grant_Custom_Access_Through_New_Team_Membership()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallLicenseAsync(TeamLicense, cancellationToken);

        var roleResponse = await CreateRoleAsync("TeamScopedRole", cancellationToken);
        roleResponse.EnsureSuccessStatusCode();
        var roleId = (await ReadJsonAsync(roleResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var teamResponse = await Client.PostAsJsonAsync(
            "/api/v1/teams",
            new
            {
                name = "TeamWithCustomAccess",
                roleIds = new[] { roleId }
            },
            cancellationToken);
        teamResponse.EnsureSuccessStatusCode();
        var teamId = (await ReadJsonAsync(teamResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var userResponse = await Client.PostAsJsonAsync(
            "/api/v1/users",
            new
            {
                name = "TeamMembershipUser",
                email = "team-membership@example.test",
                password = "Test-password-123!",
                isEnabled = true
            },
            cancellationToken);
        userResponse.EnsureSuccessStatusCode();
        var userId = (await ReadJsonAsync(userResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var removeResponse = await Client.DeleteAsync("/api/v1/license", cancellationToken);
        removeResponse.EnsureSuccessStatusCode();

        var addMemberResponse = await Client.PostAsJsonAsync(
            $"/api/v1/teams/{teamId}/members",
            new { userId },
            cancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, addMemberResponse.StatusCode);
        var problem = await ReadJsonAsync(addMemberResponse, cancellationToken);
        Assert.Equal(
            "CustomAccessControl",
            problem.GetProperty("capability").GetString());
    }

    [Fact]
    public async Task Community_Should_Allow_Adding_Member_When_Custom_Team_Access_Is_Removed_In_Same_Patch()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await InstallLicenseAsync(TeamLicense, cancellationToken);

        var roleResponse = await CreateRoleAsync("RemovedTeamScopedRole", cancellationToken);
        roleResponse.EnsureSuccessStatusCode();
        var roleId = (await ReadJsonAsync(roleResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var teamResponse = await Client.PostAsJsonAsync(
            "/api/v1/teams",
            new
            {
                name = "TeamRemovingCustomAccess",
                roleIds = new[] { roleId }
            },
            cancellationToken);
        teamResponse.EnsureSuccessStatusCode();
        var teamId = (await ReadJsonAsync(teamResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var userResponse = await Client.PostAsJsonAsync(
            "/api/v1/users",
            new
            {
                name = "TeamReductionUser",
                email = "team-reduction@example.test",
                password = "Test-password-123!",
                isEnabled = true
            },
            cancellationToken);
        userResponse.EnsureSuccessStatusCode();
        var userId = (await ReadJsonAsync(userResponse, cancellationToken))
            .GetProperty("id")
            .GetGuid();

        var removeResponse = await Client.DeleteAsync("/api/v1/license", cancellationToken);
        removeResponse.EnsureSuccessStatusCode();

        var patchContent = new StringContent(
            $$"""
              {
                "userIds": ["{{userId}}"],
                "roleIds": [],
                "resourceAccesses": []
              }
              """,
            Encoding.UTF8,
            "application/merge-patch+json");
        var patchResponse = await Client.PatchAsync(
            $"/api/v1/teams/{teamId}",
            patchContent,
            cancellationToken);

        patchResponse.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task Legacy_Business_License_Should_Report_Team_As_Effective_Edition()
    {
        var response = await InstallLicenseAsync(
            LegacyBusinessLicense,
            TestContext.Current.CancellationToken);
        var state = await ReadJsonAsync(response, TestContext.Current.CancellationToken);

        Assert.Equal("Business", state.GetProperty("licensedEdition").GetString());
        Assert.Equal("Team", state.GetProperty("effectiveEdition").GetString());
        Assert.Equal(1, state.GetProperty("licenseSchema").GetInt32());
        Assert.All(
            state.GetProperty("capabilities").EnumerateArray(),
            capability => Assert.True(capability.GetProperty("enabled").GetBoolean()));
    }

    private async Task<HttpResponseMessage> InstallLicenseAsync(
        string license,
        CancellationToken cancellationToken)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/license",
            new { license },
            cancellationToken);
        response.EnsureSuccessStatusCode();
        return response;
    }

    private Task<HttpResponseMessage> CreateRoleAsync(
        string name,
        CancellationToken cancellationToken)
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

    private static bool GetCapability(JsonElement license, LicenseCapability capability)
        => license.GetProperty("capabilities")
            .EnumerateArray()
            .Single(item => string.Equals(
                item.GetProperty("capability").GetString(),
                capability.ToString(),
                StringComparison.Ordinal))
            .GetProperty("enabled")
            .GetBoolean();

    private sealed class TestLicenseVerifier : ILicenseVerifier
    {
        public LicenseVerificationResult Verify(
            string rawLicense,
            CitadelInstanceIdentity instance,
            DateTimeOffset now)
        {
            var descriptor = rawLicense switch
            {
                TeamLicense => new LicenseDescriptor(
                    "license-team",
                    null,
                    LicenseConstants.CurrentSchema,
                    LicenseConstants.EditionTeam,
                    LicenseStatus.Valid),
                FutureTeamLicense => new LicenseDescriptor(
                    "license-future",
                    null,
                    LicenseConstants.CurrentSchema,
                    LicenseConstants.EditionTeam,
                    LicenseStatus.NotYetValid),
                UnrelatedTeamLicense => new LicenseDescriptor(
                    "license-unrelated",
                    null,
                    LicenseConstants.CurrentSchema,
                    LicenseConstants.EditionTeam,
                    LicenseStatus.Valid),
                ReplacementTeamLicense => new LicenseDescriptor(
                    "license-replacement",
                    "license-team",
                    LicenseConstants.CurrentSchema,
                    LicenseConstants.EditionTeam,
                    LicenseStatus.Valid),
                LegacyBusinessLicense => new LicenseDescriptor(
                    "license-legacy",
                    null,
                    LicenseConstants.LegacySchema,
                    LicenseConstants.EditionBusiness,
                    LicenseStatus.Valid),
                _ => null
            };

            if (descriptor is null)
            {
                return LicenseVerificationResult.Failed(
                    LicenseStatus.Invalid,
                    "LICENSE_INVALID",
                    "License is not valid.");
            }

            var payload = new LicensePayload(
                Schema: descriptor.Schema,
                Product: LicenseConstants.Product,
                Issuer: LicenseConstants.Issuer,
                Audience: LicenseConstants.Audience,
                LicenseId: descriptor.LicenseId,
                ReplacedLicenseId: descriptor.ReplacedLicenseId,
                Customer: new LicenseCustomer("customer-integration-test", "Integration Test"),
                Edition: descriptor.Edition,
                InstanceId: instance.InstanceId,
                IssuedAt: now.AddDays(-1),
                NotBefore: descriptor.Status == LicenseStatus.NotYetValid ? now.AddDays(1) : now.AddDays(-1),
                ExpiresAt: now.AddDays(30),
                GraceUntil: now.AddDays(44),
                Limits: descriptor.Schema == LicenseConstants.LegacySchema
                    ? new Dictionary<string, int>()
                    : null,
                Capabilities: descriptor.Schema == LicenseConstants.CurrentSchema
                    ? LicenseCapabilityKeys.All.Select(LicenseCapabilityKeys.GetKey).ToArray()
                    : null);
            var verified = new VerifiedLicense(
                RawLicense: rawLicense,
                Fingerprint: $"integration-test-{descriptor.LicenseId}",
                KeyId: "integration-test-key",
                Payload: payload,
                Status: descriptor.Status,
                EffectiveCapabilities: LicenseCapabilityKeys.All,
                Warnings: descriptor.Schema == LicenseConstants.LegacySchema
                    ? ["Legacy Business license mapped to Team."]
                    : []);

            return LicenseVerificationResult.Succeeded(verified);
        }
    }

    private sealed record LicenseDescriptor(
        string LicenseId,
        string? ReplacedLicenseId,
        int Schema,
        string Edition,
        LicenseStatus Status);
}
