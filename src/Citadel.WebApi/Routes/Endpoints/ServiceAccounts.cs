using Application.Features.Identity.ServiceAccounts;
using Application.Permissions;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Identity.ServiceAccounts;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints;

public static class ServiceAccounts
{
    public static async Task<Results<Ok<ServiceAccountsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [AsParameters] ServiceAccountsFilter filter,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, ServiceAccountsView.Map);
    }

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute] Guid id,
        CancellationToken cancellationToken)
        => await EndpointHandlers.HandleResult(
            await mediator.Send(new GetServiceAccount(id), cancellationToken),
            permissionEvaluator,
            ServiceAccountView.MapWithCapabilities);

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> Create(
        IMediator mediator,
        [FromBody] CreateServiceAccountInput input,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(input.ToCommand(), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute] Guid id,
        PatchServiceAccountInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<PatchServiceAccountModel> mapped =
            patchInput.Map<PatchServiceAccountInput, PatchServiceAccountModel>();
        return EndpointHandlers.HandleResult(
            await mediator.Send(new PatchServiceAccount(id, mapped), cancellationToken),
            account => ServiceAccountView.Map(account));
    }

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        [FromBody] RenameResource input,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new RenameServiceAccount(input.Id, input.Name), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<NoContent, ProblemHttpResult>> Archive(
        IMediator mediator,
        [FromBody] DeleteServiceAccountsInput input,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResultForNoContent(await mediator.Send(input.ToCommand(), cancellationToken));

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> AddRole(
        IMediator mediator,
        [FromRoute] Guid id,
        [FromBody] AddServiceAccountRoleInput input,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new AddServiceAccountRole(id, input.RoleId), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> RemoveRole(
        IMediator mediator,
        [FromRoute] Guid id,
        [FromRoute] Guid roleId,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new RemoveServiceAccountRole(id, roleId), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> AddResourceAccess(
        IMediator mediator,
        [FromRoute] Guid id,
        [FromBody] ServiceAccountResourceAccessInput input,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new AddServiceAccountResourceAccess(
                id,
                input.ResourceType,
                input.ResourceId,
                input.PermissionLevel,
                input.SpecificPermissions), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<Ok<ServiceAccountView>, ProblemHttpResult>> RemoveResourceAccess(
        IMediator mediator,
        [FromRoute] Guid id,
        [FromRoute] Guid resourceAccessId,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new RemoveServiceAccountResourceAccess(id, resourceAccessId), cancellationToken),
            account => ServiceAccountView.Map(account));

    public static async Task<Results<Ok<ServiceAccountTokensView>, ProblemHttpResult>> ListTokens(
        IMediator mediator,
        [FromRoute] Guid id,
        [AsParameters] ServiceAccountTokensFilter filter,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new GetServiceAccountTokens(id, filter.Page, filter.PageSize), cancellationToken),
            ServiceAccountTokensView.Map);

    public static async Task<Results<Ok<IEnumerable<RunAsActorUsageView>>, ProblemHttpResult>> ListUsages(
        IMediator mediator,
        [FromRoute] Guid id,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new GetServiceAccountUsages(id), cancellationToken),
            usages => usages.Select(RunAsActorUsageView.Map));

    public static async Task<Results<Created<CreatedServiceAccountTokenView>, ProblemHttpResult>> CreateToken(
        HttpContext context,
        IMediator mediator,
        [FromRoute] Guid id,
        [FromBody] CreateServiceAccountTokenInput input,
        CancellationToken cancellationToken)
    {
        context.Response.Headers.CacheControl = "no-store";
        context.Response.Headers.Pragma = "no-cache";
        var result = await mediator.Send(
            new CreateServiceAccountToken(id, input.Name, input.ExpiresAtUtc, input.NeverExpires),
            cancellationToken);
        return EndpointHandlers.HandleCreated(result, CreatedServiceAccountTokenView.Map, created => $"/api/v1/serviceAccounts/{id}/tokens/{created.Credential.Id}");
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RevokeToken(
        IMediator mediator,
        [FromRoute] Guid id,
        [FromRoute] Guid tokenId,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResultForNoContent(await mediator.Send(new RevokeServiceAccountToken(id, tokenId), cancellationToken));

    public static async Task<Results<Ok<ServiceAccountLimitsView>, ProblemHttpResult>> GetLimits(
        IMediator mediator,
        CancellationToken cancellationToken)
        => EndpointHandlers.HandleResult(
            await mediator.Send(new GetServiceAccountLimits(), cancellationToken),
            ServiceAccountLimitsView.Map);
}
