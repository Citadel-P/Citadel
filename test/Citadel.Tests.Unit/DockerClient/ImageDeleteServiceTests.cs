using Hosting.DockerClient;
using Hosting.DockerClient.HttpClient;
using Hosting.DockerClient.Models.Images;
using Hosting.DockerClient.Services;
using Moq;
using IDockerClient = Hosting.DockerClient.IDockerClient;

namespace Tests.Unit.DockerClient;

public sealed class ImageDeleteServiceTests
{
    [Fact]
    public async Task DeleteAsync_ShouldNotStartNextDaemonCallUntilCurrentCallCompletes()
    {
        var firstCompletion = new TaskCompletionSource<ICollection<ImageDeleteResponseItem>>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var dockerClient = new Mock<IDockerClient>(MockBehavior.Strict);
        dockerClient
            .Setup(client => client.ImageDelete(
                "image-one",
                false,
                false,
                It.IsAny<CancellationToken>()))
            .Returns(firstCompletion.Task);
        dockerClient
            .Setup(client => client.ImageDelete(
                "image-two",
                false,
                false,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([new ImageDeleteResponseItem { Deleted = "image-two" }]);

        var service = new ImageService(
            dockerClient.Object,
            Mock.Of<IStreamService>(),
            Mock.Of<IDockerConnection>());

        var deletion = service.DeleteAsync(
            new DeleteImageCommand(["image-one", "image-two"], false, false),
            TestContext.Current.CancellationToken);

        dockerClient.Verify(
            client => client.ImageDelete("image-one", false, false, It.IsAny<CancellationToken>()),
            Times.Once);
        dockerClient.Verify(
            client => client.ImageDelete("image-two", false, false, It.IsAny<CancellationToken>()),
            Times.Never);

        firstCompletion.SetResult([new ImageDeleteResponseItem { Deleted = "image-one" }]);
        var result = await deletion;

        Assert.True(result.IsSuccess(out var response));
        Assert.Equal(2, response.Items.Count());
        dockerClient.Verify(
            client => client.ImageDelete("image-two", false, false, It.IsAny<CancellationToken>()),
            Times.Once);
    }
}
