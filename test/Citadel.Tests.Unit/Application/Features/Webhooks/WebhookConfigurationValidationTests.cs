using Application.Features.GitRepositories.Commands;
using Domain;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using FluentValidation;

namespace Tests.Unit.Application.Features.Webhooks;

public sealed class WebhookConfigurationValidationTests
{
    [Fact]
    public void GetAuthenticationError_ShouldAcceptOnlySupportedProviderAndAuthenticationPairs()
    {
        Assert.Null(WebhookConfigurationValidation.GetAuthenticationError(
            WebhookProvider.Generic,
            WebhookAuthScheme.BearerToken,
            "shared-secret"));
        Assert.Equal(
            "Generic webhooks require shared-secret authentication using the Authorization: Bearer header.",
            WebhookConfigurationValidation.GetAuthenticationError(
                WebhookProvider.Generic,
                WebhookAuthScheme.GitHubHmacSha256,
                "shared-secret"));
        Assert.Equal(
            "Generic webhooks require a shared secret.",
            WebhookConfigurationValidation.GetAuthenticationError(
                WebhookProvider.Generic,
                WebhookAuthScheme.BearerToken,
                null));
        Assert.Equal(
            "The webhook provider is not supported.",
            WebhookConfigurationValidation.GetAuthenticationError(
                (WebhookProvider)999,
                WebhookAuthScheme.BearerToken,
                "shared-secret"));
        Assert.Equal(
            "The webhook authentication scheme is not supported.",
            WebhookConfigurationValidation.GetAuthenticationError(
                WebhookProvider.Generic,
                (WebhookAuthScheme)999,
                "shared-secret"));
    }

    [Fact]
    public void RepositoryValidators_ShouldRejectOversizedWebhookFieldsOnCreateAndPatch()
    {
        var webhook = new RepoWebhookConfig(
            Enabled: true,
            Provider: WebhookProvider.Generic,
            AuthScheme: WebhookAuthScheme.BearerToken,
            Secret: new string('s', 257),
            BranchFilter: new string('b', 257));
        var create = new CreateGitRepository(
            "repository",
            null,
            "https://example.test/repository.git",
            "main",
            null,
            webhook);
        var repository = new GitRepository(
            "repository",
            null,
            "https://example.test/repository.git",
            "main",
            null,
            Guid.CreateVersion7(),
            webhook: webhook);

        var createResult = new CreateGitRepository.Validator().Validate(create);
        var patchResult = new PatchGitRepository.GitRepositoryValidator().Validate(repository);

        Assert.Contains(createResult.Errors, error => error.PropertyName == "Webhook.Secret");
        Assert.Contains(createResult.Errors, error => error.PropertyName == "Webhook.BranchFilter");
        Assert.Contains(patchResult.Errors, error => error.PropertyName == "Webhook.Secret");
        Assert.Contains(patchResult.Errors, error => error.PropertyName == "Webhook.BranchFilter");
    }
}
