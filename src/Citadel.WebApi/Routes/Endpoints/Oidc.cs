using Application.Features.Oidc.Commands;
using Application.Features.Oidc.Queries;
using Application.Models;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Oidc;
using WebApi.Transport;

namespace WebApi.Routes.Endpoints;

public static class Oidc
{
    public static async Task<IResult> BeginLogin(
        IMediator mediator,
        IConfiguration configuration,
        ICitadelPublicEndpoints publicEndpoints,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        [FromQuery] string? returnUrl,
        CancellationToken cancellationToken)
    {
        var redirectUri = publicEndpoints.BuildOidcCallback(id);
        var result = await mediator.Send(
            new BeginOidcLogin(
                id,
                redirectUri,
                NormalizeReturnUrl(publicEndpoints, returnUrl),
                GetAllowedReturnOrigins(configuration)),
            cancellationToken);

        return result.IsSuccess(out var loginStart)
            ? TypedResults.Redirect(loginStart.AuthorizationUrl)
            : EndpointHandlers.HandleResult(result);
    }

    public static async Task<IResult> CompleteLogin(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        [FromQuery] string? code,
        [FromQuery] string? state,
        [FromQuery] string? error,
        [FromQuery(Name = "error_description")] string? errorDescription,
        CancellationToken cancellationToken)
    {
        if (!string.IsNullOrWhiteSpace(error))
            return TypedResults.Problem(errorDescription ?? error, statusCode: StatusCodes.Status400BadRequest);

        if (string.IsNullOrWhiteSpace(code) || string.IsNullOrWhiteSpace(state))
            return TypedResults.Problem("Missing OIDC callback parameters.", statusCode: StatusCodes.Status400BadRequest);

        var redirectUri = publicEndpoints.BuildOidcCallback(id);
        var result = await mediator.Send(new CompleteOidcLogin(id, code, state, redirectUri), cancellationToken);
        if (!result.IsSuccess(out var loginComplete))
            return EndpointHandlers.HandleResult(result);

        return TypedResults.Redirect(loginComplete.ReturnUrl);
    }

    public static async Task<Results<Ok<OidcLoginProvidersView>, ProblemHttpResult>> ListLoginProviders(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetEnabledOidcLoginProviders(), cancellationToken);
        return EndpointHandlers.HandleResult(result, OidcLoginProvidersView.Map);
    }

    public static async Task<Results<Ok<OidcProvidersView>, ProblemHttpResult>> ListProviders(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetOidcProviders(), cancellationToken);
        return EndpointHandlers.HandleResult(result, OidcProvidersView.Map);
    }

    public static async Task<Results<Ok<OidcProviderView>, ProblemHttpResult>> GetProvider(
        IMediator mediator,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetOidcProvider(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, OidcProviderView.Map);
    }

    public static async Task<Results<Ok<OidcProviderView>, ProblemHttpResult>> CreateProvider(
        IMediator mediator,
        [FromBody] OidcProviderInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateOidcProvider(input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResult(result, OidcProviderView.Map);
    }

    public static async Task<Results<Ok<OidcProviderView>, ProblemHttpResult>> UpdateProvider(
        IMediator mediator,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        UpdateOidcProviderPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateOidcProviderInput(),
            ApplicationJsonContext.Default.UpdateOidcProviderInput);

        var result = await mediator.Send(
            new UpdateOidcProvider(
                id,
                input.ToModel(),
                patchInput.ContainsProperty("description"),
                patchInput.ContainsProperty("allowedEmailDomains"),
                patchInput.ContainsProperty("requiredClaimName"),
                patchInput.ContainsProperty("requiredClaimValues"),
                patchInput.ContainsProperty("defaultRoleId")),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, OidcProviderView.Map);
    }

    public static async Task<Results<Ok<OidcProviderView>, ProblemHttpResult>> RenameProvider(
        IMediator mediator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameOidcProvider(renameResource.Id, renameResource.Name), cancellationToken);

        return EndpointHandlers.HandleResult(result, OidcProviderView.Map);
    }

    public static async Task<Results<Ok<OidcProviderView>, ProblemHttpResult>> UpdateProviderMetadata(
        IMediator mediator,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new PatchResourceMetadata(string.Empty, []),
            ApplicationJsonContext.Default.PatchResourceMetadata);
        var result = await mediator.Send(new PatchOidcProviderMetadata(id, input.Description), cancellationToken);

        return EndpointHandlers.HandleResult(result, OidcProviderView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteProvider(
        IMediator mediator,
        [FromRoute][Description("OIDC provider ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteOidcProvider(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<OidcDiscoveryResultView>, ProblemHttpResult>> TestDiscovery(
        IMediator mediator,
        [FromBody] TestOidcProviderDiscoveryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new TestOidcProviderDiscovery(input.ProviderId, input.Issuer),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, OidcDiscoveryResultView.Map);
    }

    private static string NormalizeReturnUrl(
        ICitadelPublicEndpoints publicEndpoints,
        string? returnUrl)
        => string.IsNullOrWhiteSpace(returnUrl)
            ? publicEndpoints.BuildApplicationRoot()
            : returnUrl;

    private static IReadOnlyCollection<string> GetAllowedReturnOrigins(IConfiguration configuration)
        => configuration.GetSection("Cors").Get<string[]>() ?? [];
}
