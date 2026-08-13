using Application.Features.Identity.ServiceAccounts;
using Application.Permissions;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Models;
using Hosting.Common.MergePatch;
using Microsoft.AspNetCore.Mvc;
using System.Reflection;
using System.Text.Json;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Identity.ServiceAccounts;

public sealed record ServiceAccountResourceAccessInput(
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions)
{
    internal ResourceAccessView ToModel() => new(ResourceType, ResourceId, null, PermissionLevel, SpecificPermissions);
}

public sealed record CreateServiceAccountInput(
    string Name,
    string? Description,
    bool IsEnabled = true,
    IEnumerable<Guid>? TeamIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<ServiceAccountResourceAccessInput>? ResourceAccesses = null)
{
    internal CreateServiceAccount ToCommand() => new(
        Name,
        Description,
        IsEnabled,
        TeamIds,
        RoleIds,
        ResourceAccesses?.Select(x => x.ToModel()));
}

public sealed record PatchServiceAccountInput(
    string? Description,
    bool? IsEnabled);

public sealed class PatchServiceAccountInputPatchDocument : JsonMergePatchDocument<PatchServiceAccountInput>
{
    public static async ValueTask<PatchServiceAccountInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var document = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchServiceAccountInputPatchDocument { Patch = document.RootElement.Clone() };
    }
}

public sealed record AddServiceAccountRoleInput(Guid RoleId);

public sealed record DeleteServiceAccountsInput(IEnumerable<Guid> Ids)
{
    internal ArchiveServiceAccounts ToCommand() => new(Ids);
}

public sealed record CreateServiceAccountTokenInput(
    string Name,
    DateTimeOffset? ExpiresAtUtc = null,
    bool NeverExpires = false);

public sealed record ServiceAccountsFilter(
    [FromQuery] string Name = null,
    [FromQuery] int Page = 1,
    [FromQuery] int PageSize = 50,
    [FromQuery] bool IncludeArchived = false)
{
    internal GetServiceAccounts ToQuery() => new(Page, PageSize, Name, IncludeArchived);
}

public sealed record ServiceAccountTokensFilter(
    [FromQuery] int Page = 1,
    [FromQuery] int PageSize = 50);

public sealed record ServiceAccountView(
    Guid Id,
    string Name,
    string? Description,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    DateTime UpdatedAt,
    DateTime? ArchivedAtUtc,
    int ActiveTokenCount,
    DateTime? LastUsedAtUtc,
    IEnumerable<ResourceInfo>? Teams,
    IEnumerable<ResourceInfo>? Roles,
    IEnumerable<ResourceAccessView>? ResourceAccesses,
    ServiceAccountCapabilities? Capabilities = null)
{
    internal static ServiceAccountView Map(
        ServiceAccountDetails account,
        ServiceAccountCapabilities? capabilities = null) => new(
        account.Id,
        account.Name,
        account.Description,
        account.ActorId,
        account.IsEnabled,
        account.CreatedAt,
        account.CreatedByActorId,
        account.UpdatedAt,
        account.ArchivedAtUtc,
        account.ActiveTokenCount,
        account.LastUsedAtUtc,
        account.Teams,
        account.Roles,
        account.ResourceAccesses,
        capabilities);

    internal static async Task<ServiceAccountView> MapWithCapabilities(
        ServiceAccountDetails account,
        IPermissionEvaluator permissionEvaluator)
        => Map(
            account,
            CapabilityMapper.ToServiceAccountCapabilities(
                await permissionEvaluator.EvaluateAsync(account.Id, ResourceType.ServiceAccount)));
}

public sealed record ServiceAccountsView(
    PagedResultView<ServiceAccountView> PagedResult,
    ResourceCapabilities Capabilities)
{
    internal static async Task<ServiceAccountsView> Map(
        PagedResult<ServiceAccountDetails> result,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(ResourceType.ServiceAccount);
        var itemPermissions = await permissionEvaluator.EvaluateAsync(
            result.Items.Select(static account => account.Id).ToArray(),
            ResourceType.ServiceAccount);
        return new(
            new PagedResultView<ServiceAccountView>(
                result.Items.Select(account => ServiceAccountView.Map(
                    account,
                    CapabilityMapper.ToServiceAccountCapabilities(
                        itemPermissions.GetValueOrDefault(account.Id)))),
                result.TotalCount,
                result.Page,
                result.PageSize),
            CapabilityMapper.ToResourceCapabilities(permissions));
    }
}

public sealed record ServiceAccountTokenView(
    Guid Id,
    string Name,
    string Hint,
    DateTime? ExpiresAtUtc,
    DateTime? LastUsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid? RevokedByActorId,
    Guid CreatedByActorId,
    string CreatedByName,
    DateTime CreatedAtUtc)
{
    internal static ServiceAccountTokenView Map(ServiceAccountTokenDetails token) => new(
        token.Id,
        token.Name,
        $"cit_sa_{token.Id:N}"[..15],
        token.ExpiresAtUtc,
        token.LastUsedAtUtc,
        token.RevokedAtUtc,
        token.RevokedByActorId,
        token.CreatedByActorId,
        token.CreatedByName,
        token.CreatedAtUtc);
}

public sealed record ServiceAccountTokensView(PagedResultView<ServiceAccountTokenView> PagedResult)
{
    internal static ServiceAccountTokensView Map(PagedResult<ServiceAccountTokenDetails> result) => new(
        new PagedResultView<ServiceAccountTokenView>(
            result.Items.Select(ServiceAccountTokenView.Map),
            result.TotalCount,
            result.Page,
            result.PageSize));
}

public sealed record CreatedServiceAccountTokenView(
    Guid Id,
    string Name,
    string Hint,
    DateTime? ExpiresAtUtc,
    DateTime? LastUsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid? RevokedByActorId,
    Guid CreatedByActorId,
    string CreatedByName,
    DateTime CreatedAtUtc,
    string Token)
{
    internal static CreatedServiceAccountTokenView Map(CreatedServiceAccountToken created)
    {
        var credential = ServiceAccountTokenView.Map(created.Credential);
        return new(
            credential.Id,
            credential.Name,
            credential.Hint,
            credential.ExpiresAtUtc,
            credential.LastUsedAtUtc,
            credential.RevokedAtUtc,
            credential.RevokedByActorId,
            credential.CreatedByActorId,
            credential.CreatedByName,
            credential.CreatedAtUtc,
            created.Token);
    }
}

public sealed record ServiceAccountLimitsView(
    int DefaultTokenLifetimeDays,
    int MaximumTokenLifetimeDays,
    int MaximumActiveTokensPerAccount)
{
    internal static ServiceAccountLimitsView Map(ServiceAccountLimits limits)
        => new(limits.DefaultTokenLifetimeDays, limits.MaximumTokenLifetimeDays, limits.MaximumActiveTokensPerAccount);
}

public sealed record RunAsActorUsageView(Guid Id, string Name, ResourceType ResourceType, bool IsActive)
{
    internal static RunAsActorUsageView Map(RunAsActorUsage usage)
        => new(usage.Id, usage.Name, usage.ResourceType, usage.IsActive);
}
