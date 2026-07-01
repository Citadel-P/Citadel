using Application.Features.Configuration.Commands;
using Application.Features.Configuration.Models;
using Application.Features.Configuration.Queries;
using Application.Permissions;
using Hosting.Extensions;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.ResourceBindings;
using Domain.Entities.ResourceBindings;
using Domain;

namespace WebApi.Routes.Endpoints;

public static class ResourceBindings
{
    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> GetGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGlobalResourceBindings(), cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Binding, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> ReplaceGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] ReplaceResourceBindingsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new ReplaceGlobalResourceBindings(input.Entries),
            cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Binding, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> GetResource(
        IMediator mediator,
        [FromRoute][Description("Resource binding scope")] ResourceBindingScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ResourceBindingScope.Stack => await mediator.Send(new GetStackResourceBindings(resourceId), cancellationToken),
            ResourceBindingScope.Deployment => await mediator.Send(new GetDeploymentResourceBindings(resourceId), cancellationToken),
            ResourceBindingScope.Global => Result.Failure<ResourceBindingsResult>(new BadRequestError("Global resource bindings do not target a resource.")),
            _ => Result.Failure<ResourceBindingsResult>(new BadRequestError($"Unsupported resource binding scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> ReplaceResource(
        IMediator mediator,
        [FromRoute][Description("Resource binding scope")] ResourceBindingScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        [FromBody] ReplaceResourceBindingsInput input,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ResourceBindingScope.Stack => await mediator.Send(new ReplaceStackResourceBindings(resourceId, input.Entries), cancellationToken),
            ResourceBindingScope.Deployment => await mediator.Send(new ReplaceDeploymentResourceBindings(resourceId, input.Entries), cancellationToken),
            ResourceBindingScope.Global => Result.Failure<ResourceBindingsResult>(new BadRequestError("Global resource bindings do not target a resource.")),
            _ => Result.Failure<ResourceBindingsResult>(new BadRequestError($"Unsupported resource binding scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data));
    }

    public static async Task<Results<Ok<SecretDefinitionsView>, ProblemHttpResult>> ListSecrets(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery][Description("Optional resource binding scope")] ResourceBindingScope? scope,
        [FromQuery][Description("Optional resource ID")] Guid? resourceId,
        CancellationToken cancellationToken)
    {
        var result = (scope, resourceId) switch
        {
            (null, null) => await mediator.Send(new GetSecretDefinitions(), cancellationToken),
            (ResourceBindingScope.Stack, { } id) => await mediator.Send(new GetStackSecretDefinitions(id), cancellationToken),
            (ResourceBindingScope.Deployment, { } id) => await mediator.Send(new GetDeploymentSecretDefinitions(id), cancellationToken),
            (ResourceBindingScope.Global, _) => Result.Failure<IReadOnlyList<SecretDefinition>>(new BadRequestError("Global resource bindings do not target a resource.")),
            ({ }, null) => Result.Failure<IReadOnlyList<SecretDefinition>>(new BadRequestError("resourceId must be provided when scope is provided.")),
            _ => Result.Failure<IReadOnlyList<SecretDefinition>>(new BadRequestError($"Unsupported resource binding scope '{scope}'."))
        };
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Binding, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => SecretDefinitionsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> CreateInternalSecret(
        IMediator mediator,
        [FromBody] CreateInternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateInternalSecret(input.Name, input.Value), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> CreateExternalSecret(
        IMediator mediator,
        [FromBody] CreateExternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateExternalSecret(
                input.Name,
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> UpdateExternalSecret(
        IMediator mediator,
        [FromRoute][Description("Secret definition ID")] Guid id,
        [FromBody] UpdateExternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new UpdateExternalSecret(
                id,
                input.Name,
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<Ok<ExternalSecretTestResultView>, ProblemHttpResult>> TestExternalSecret(
        IMediator mediator,
        [FromBody] TestExternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new TestExternalSecret(
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, ExternalSecretTestResultView.Map);
    }

    public static async Task<Results<Ok<SecretProviderConnectionTestResultView>, ProblemHttpResult>> TestVaultKvV2SecretProviderConnection(
        IMediator mediator,
        [FromBody] TestVaultKvV2SecretProviderConnectionInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new TestVaultKvV2SecretProviderConnection(
                input.ProviderId,
                input.Name,
                input.Address,
                input.MountPath,
                input.Token),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretProviderConnectionTestResultView.Map);
    }

    public static async Task<Results<Ok<SecretProvidersView>, ProblemHttpResult>> ListSecretProviders(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSecretProviders(), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretProvidersView.Map);
    }

    public static async Task<Results<Ok<SecretProviderView>, ProblemHttpResult>> CreateVaultKvV2SecretProvider(
        IMediator mediator,
        [FromBody] CreateVaultKvV2SecretProviderInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateVaultKvV2SecretProvider(
                input.Name,
                input.Address,
                input.MountPath,
                input.Token),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretProviderView.Map);
    }

    public static async Task<Results<Ok<SecretProviderView>, ProblemHttpResult>> UpdateVaultKvV2SecretProvider(
        IMediator mediator,
        [FromRoute][Description("Secret provider ID")] Guid id,
        [FromBody] UpdateVaultKvV2SecretProviderInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new UpdateVaultKvV2SecretProvider(
                id,
                input.Name,
                input.Address,
                input.MountPath,
                input.Token),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretProviderView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteSecretProvider(
        IMediator mediator,
        [FromRoute][Description("Secret provider ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteSecretProvider(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
