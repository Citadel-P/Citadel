using Application.Features.GitRepositories.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Features.GitRepositories;

public sealed class DeleteGitRepositoriesTests
{
    [Fact]
    public async Task Handle_ShouldSucceedAfterCommit_WhenNotificationsAndCacheCleanupFail()
    {
        var repository = CreateRepository();
        var mainRepositories = new Mock<IGitReposRepository>();
        mainRepositories
            .Setup(value => value.GetAllAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([repository]);
        mainRepositories
            .Setup(value => value.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<GitReposStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                true,
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var mainUnitOfWork = new Mock<IUnitOfWork>();
        mainUnitOfWork.SetupGet(value => value.GitRepositories).Returns(mainRepositories.Object);

        var deleteRepositories = new Mock<IGitReposRepository>();
        deleteRepositories
            .Setup(value => value.RemoveRangeAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var deleteUnitOfWork = CreateDeleteUnitOfWork(deleteRepositories.Object);
        using var services = new ServiceCollection()
            .AddSingleton<IUnitOfWork>(deleteUnitOfWork.Object)
            .BuildServiceProvider();

        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(value => value.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.FromException(new InvalidOperationException("queue unavailable")));
        var cache = new Mock<IRepoCacheManager>();
        cache
            .Setup(value => value.DeleteCacheAsync(repository, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new IOException("cache unavailable"));
        var handler = CreateHandler(
            mainUnitOfWork.Object,
            services.GetRequiredService<IServiceScopeFactory>(),
            cache.Object,
            notificationQueue.Object);

        var result = await handler.Handle(
            new DeleteGitRepositories([repository.Id]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        deleteRepositories.Verify(value => value.RemoveRangeAsync(
            It.Is<IEnumerable<Guid>>(ids => ids.SequenceEqual(new[] { repository.Id })),
            It.IsAny<CancellationToken>()), Times.Once);
        cache.Verify(value => value.DeleteCacheAsync(
            repository,
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Handle_ShouldRollbackClaim_WhenDeleteLosesConcurrencyRace()
    {
        var repository = CreateRepository();
        var mainRepositories = new Mock<IGitReposRepository>();
        mainRepositories
            .Setup(value => value.GetAllAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([repository]);
        mainRepositories
            .Setup(value => value.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<GitReposStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                true,
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var mainUnitOfWork = new Mock<IUnitOfWork>();
        mainUnitOfWork.SetupGet(value => value.GitRepositories).Returns(mainRepositories.Object);

        var deleteRepositories = new Mock<IGitReposRepository>();
        deleteRepositories
            .Setup(value => value.RemoveRangeAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var deleteUnitOfWork = CreateDeleteUnitOfWork(deleteRepositories.Object);
        using var services = new ServiceCollection()
            .AddSingleton<IUnitOfWork>(deleteUnitOfWork.Object)
            .BuildServiceProvider();
        var handler = CreateHandler(
            mainUnitOfWork.Object,
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IRepoCacheManager>(),
            Mock.Of<INotificationQueue>());

        var result = await handler.Handle(
            new DeleteGitRepositories([repository.Id]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        Assert.Equal(GitReposStatus.Created, repository.Status);
        Assert.Equal(ResourceControlState.Idle, repository.ControlState);
        mainRepositories.Verify(value => value.UpdateProcessingAsync(
            repository.Id,
            GitReposStatus.Created,
            ResourceControlState.Idle,
            null,
            repository.RowVersion + 1,
            true,
            null,
            It.Is<CancellationToken>(token => !token.IsCancellationRequested)), Times.Once);
        deleteUnitOfWork.Verify(value => value.RollbackAsync(), Times.Once);
    }

    private static DeleteGitRepositoriesHandler CreateHandler(
        IUnitOfWork unitOfWork,
        IServiceScopeFactory scopeFactory,
        IRepoCacheManager cache,
        INotificationQueue notificationQueue)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(value => value.ActorId).Returns(Constants.SystemId);
        var userContext = new Mock<IUserContextAccessor>();
        userContext.SetupGet(value => value.Current).Returns(user.Object);
        var lifetime = new Mock<IHostApplicationLifetime>();
        lifetime.SetupGet(value => value.ApplicationStopping).Returns(CancellationToken.None);

        return new DeleteGitRepositoriesHandler(
            unitOfWork,
            scopeFactory,
            cache,
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IGitRepositoryStreamManager>(),
            notificationQueue,
            userContext.Object,
            lifetime.Object,
            Mock.Of<ILogger<DeleteGitRepositoriesHandler>>());
    }

    private static Mock<IUnitOfWork> CreateDeleteUnitOfWork(IGitReposRepository repositories)
    {
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(value => value.AddAsync(
                It.IsAny<global::Domain.Entities.Activities.ActivityEvent>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var actors = new Mock<IActorRepository>();
        actors
            .Setup(value => value.GetById(
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((global::Domain.Entities.Identity.Actor?)null);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.GitRepositories).Returns(repositories);
        unitOfWork.SetupGet(value => value.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.SetupGet(value => value.Actors).Returns(actors.Object);
        return unitOfWork;
    }

    private static GitRepository CreateRepository()
        => new(
            name: "repo",
            description: null,
            url: "https://github.com/example/repo.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
}
