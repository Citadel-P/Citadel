using Application.Features.ResourceBindings.Commands;
using Application.Features.ResourceBindings.Models;
using Application.Features.ResourceBindings.Queries;
using Application.Models;
using Application.Permissions;
using Hosting.Common.ErrorTypes;
using Hosting.Extensions;
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

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> CreateGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] ResourceBindingInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateGlobalResourceBinding(input), cancellationToken);
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

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> CreateResource(
        IMediator mediator,
        [FromRoute][Description("Resource binding scope")] ResourceBindingScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        [FromBody] ResourceBindingInput input,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ResourceBindingScope.Stack => await mediator.Send(new CreateStackResourceBinding(resourceId, input), cancellationToken),
            ResourceBindingScope.Deployment => await mediator.Send(new CreateDeploymentResourceBinding(resourceId, input), cancellationToken),
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
        UpdateExternalSecretPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateExternalSecretInput(null, null, null, null, null),
            ApplicationJsonContext.Default.UpdateExternalSecretInput);

        var result = await mediator.Send(
            new UpdateExternalSecret(
                id,
                input.Name,
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion,
                patchInput.ContainsProperty("externalVersion")),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteSecretDefinition(
        IMediator mediator,
        [FromRoute][Description("Secret definition ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteSecretDefinition(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
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
        UpdateVaultKvV2SecretProviderPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateVaultKvV2SecretProviderInput(null, null, null, null),
            ApplicationJsonContext.Default.UpdateVaultKvV2SecretProviderInput);

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

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> UpdateGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] UpdateResourceBindingInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new UpdateGlobalResourceBinding(ToCommandInput(input)), cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Binding, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> DeleteGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Resource binding ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteGlobalResourceBinding(id), cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Binding, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> UpdateResource(
        IMediator mediator,
        [FromRoute][Description("Resource binding scope")] ResourceBindingScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        [FromBody] UpdateResourceBindingInput input,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ResourceBindingScope.Stack => await mediator.Send(new UpdateStackResourceBinding(resourceId, ToCommandInput(input)), cancellationToken),
            ResourceBindingScope.Deployment => await mediator.Send(new UpdateDeploymentResourceBinding(resourceId, ToCommandInput(input)), cancellationToken),
            ResourceBindingScope.Global => Result.Failure<ResourceBindingsResult>(new BadRequestError("Global resource bindings do not target a resource.")),
            _ => Result.Failure<ResourceBindingsResult>(new BadRequestError($"Unsupported resource binding scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data));
    }

    public static async Task<Results<Ok<ResourceBindingsView>, ProblemHttpResult>> DeleteResource(
        IMediator mediator,
        [FromRoute][Description("Resource binding scope")] ResourceBindingScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        [FromRoute][Description("Resource binding ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ResourceBindingScope.Stack => await mediator.Send(new DeleteStackResourceBinding(resourceId, id), cancellationToken),
            ResourceBindingScope.Deployment => await mediator.Send(new DeleteDeploymentResourceBinding(resourceId, id), cancellationToken),
            ResourceBindingScope.Global => Result.Failure<ResourceBindingsResult>(new BadRequestError("Global resource bindings do not target a resource.")),
            _ => Result.Failure<ResourceBindingsResult>(new BadRequestError($"Unsupported resource binding scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, data => ResourceBindingsView.Map(data));
    }

    private static UpdateResourceBindingInputModel ToCommandInput(UpdateResourceBindingInput input)
        => new(
            input.Id,
            input.Name,
            input.Kind,
            input.Value,
            input.SecretId,
            input.SecretDeliveryMode,
            input.TargetPath);
}
