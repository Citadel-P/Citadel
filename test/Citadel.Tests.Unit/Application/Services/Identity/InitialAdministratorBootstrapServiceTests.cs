using System.Text;
using Application.Services.Identity;

namespace Tests.Unit.Application.Services.Identity;

public sealed class InitialAdministratorBootstrapServiceTests
{
    [Fact]
    public async Task ReadPasswordAsync_ShouldReadUtf8PasswordAndTrimOneTrailingNewline()
    {
        var path = Path.GetTempFileName();
        try
        {
            await File.WriteAllTextAsync(
                path,
                "\uFEFFcorrect-horse-battery-staple\r\n",
                new UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
                TestContext.Current.CancellationToken);

            var password = await InitialAdministratorBootstrapService.ReadPasswordAsync(
                path,
                TestContext.Current.CancellationToken);

            Assert.Equal("correct-horse-battery-staple", password);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task ReadPasswordAsync_ShouldRejectContentLargerThanOneKiB()
    {
        var path = Path.GetTempFileName();
        try
        {
            await File.WriteAllBytesAsync(
                path,
                new byte[1025],
                TestContext.Current.CancellationToken);

            var exception = await Assert.ThrowsAsync<InvalidOperationException>(
                () => InitialAdministratorBootstrapService.ReadPasswordAsync(
                    path,
                    TestContext.Current.CancellationToken));

            Assert.Contains("exceeds the 1 KiB limit", exception.Message);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task ReadPasswordAsync_ShouldRejectDirectories()
    {
        var path = Path.Combine(
            Path.GetTempPath(),
            $"citadel-bootstrap-password-{Guid.NewGuid():N}");
        Directory.CreateDirectory(path);
        try
        {
            var exception = await Assert.ThrowsAsync<InvalidOperationException>(
                () => InitialAdministratorBootstrapService.ReadPasswordAsync(
                    path,
                    TestContext.Current.CancellationToken));

            Assert.Contains("regular file", exception.Message);
        }
        finally
        {
            Directory.Delete(path);
        }
    }
}
