using Application.Configs;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Backups;

public sealed class BackupRepositoryDestinationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeResticProcessRunner restic = new();
    private readonly string testRoot = Path.Combine(Path.GetTempPath(), $"citadel-backups-test-{Guid.NewGuid():N}");

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<IResticProcessRunner>(restic);
        services.Configure<BackupOptions>(options =>
        {
            options.Enabled = true;
            options.ResticPath = "restic-test";
            options.WorkingDirectory = Path.Combine(testRoot, "work");
            options.AllowedCorePaths = [Path.Combine(testRoot, "repositories")];
            options.RepositoryLeaseSeconds = 60;
            options.DefaultTimeoutSeconds = 30;
            options.MaxLogLineBytes = 4096;
        });
    }

    [Fact]
    public async Task ValidateAsync_ShouldRunSnapshotsAndPersistReadyStatus()
    {
        var repositoryPath = Path.Combine(testRoot, "repositories", "ready");
        var repository = await CreateFileSystemRepositoryAsync("backup-ready", repositoryPath);
        restic.Enqueue(exitCode: 0, stdout: "[]");

        var service = Services.GetRequiredService<IBackupRepositoryDestinationService>();
        var result = await service.ValidateAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var validation), ErrorMessages(result.Errors));
        Assert.Equal(BackupRepositoryValidationStatus.Ready, validation.Status);

        var call = Assert.Single(restic.Calls);
        Assert.Equal("restic-test", call.Command.FileName);
        Assert.Equal(["snapshots", "--json"], call.Command.Arguments);
        Assert.Equal(Path.GetFullPath(repositoryPath), call.Command.Environment["RESTIC_REPOSITORY"]);
        Assert.False(call.Command.Environment.ContainsKey("RESTIC_PASSWORD"));
        Assert.True(call.PasswordFileExisted);
        Assert.Equal("restic-password", call.PasswordFileContent);
        Assert.False(File.Exists(call.PasswordFilePath));
        Assert.Contains("restic-password", call.Command.RedactionValues);
        Assert.Contains(Path.GetFullPath(repositoryPath), call.Command.RedactionValues);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRepository = await uow.BackupRepositories.GetAsync(repository.Id, TestContext.Current.CancellationToken);
        var storedValidation = await uow.BackupRepositoryValidations.GetAsync(
            repository.Id,
            BackupExecutionLocation.Core,
            null,
            TestContext.Current.CancellationToken);

        Assert.NotNull(storedRepository);
        Assert.Equal(BackupRepositoryStatus.Ready, storedRepository.Status);
        Assert.NotNull(storedValidation);
        Assert.Equal(BackupRepositoryValidationStatus.Ready, storedValidation.Status);
        Assert.Null(storedValidation.LastErrorCode);
    }

    [Fact]
    public async Task InitializeAsync_ShouldInitializeUninitializedRepositoryAndReleaseLease()
    {
        var repository = await CreateFileSystemRepositoryAsync(
            "backup-init",
            Path.Combine(testRoot, "repositories", "init"));
        restic.Enqueue(exitCode: 1, stderr: "config file does not exist. Is there a repository at this location?");
        restic.Enqueue(exitCode: 0, stdout: "{}");
        restic.Enqueue(exitCode: 0, stdout: "[]");

        var service = Services.GetRequiredService<IBackupRepositoryDestinationService>();
        var result = await service.InitializeAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(), ErrorMessages(result.Errors));
        Assert.Equal(3, restic.Calls.Count);
        Assert.Equal(["snapshots", "--json"], restic.Calls[0].Command.Arguments);
        Assert.Equal(["init", "--json"], restic.Calls[1].Command.Arguments);
        Assert.Equal(["snapshots", "--json"], restic.Calls[2].Command.Arguments);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRepository = await uow.BackupRepositories.GetAsync(repository.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(storedRepository);
        Assert.Equal(BackupRepositoryStatus.Ready, storedRepository.Status);

        var canAcquireAfterRelease = await uow.BackupRepositoryLeases.TryAcquireAsync(
            repository.Id,
            "Test",
            Guid.CreateVersion7(),
            DateTimeOffset.UtcNow.AddMinutes(1),
            DateTimeOffset.UtcNow,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        Assert.True(canAcquireAfterRelease);
    }

    [Fact]
    public async Task CheckAndPruneAsync_ShouldPersistMaintenanceTimestamps()
    {
        var repository = await CreateFileSystemRepositoryAsync(
            "backup-maintenance",
            Path.Combine(testRoot, "repositories", "maintenance"));
        restic.Enqueue(exitCode: 0, stdout: "[]");
        restic.Enqueue(exitCode: 0, stdout: "[]");

        var service = Services.GetRequiredService<IBackupRepositoryDestinationService>();
        var check = await service.CheckAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);
        var prune = await service.PruneAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);

        Assert.True(check.IsSuccess(), ErrorMessages(check.Errors));
        Assert.True(prune.IsSuccess(), ErrorMessages(prune.Errors));
        Assert.Equal(["check", "--json"], restic.Calls[0].Command.Arguments);
        Assert.Equal(["prune", "--json"], restic.Calls[1].Command.Arguments);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedRepository = await uow.BackupRepositories.GetAsync(repository.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(storedRepository);
        Assert.NotNull(storedRepository.LastCheckedAt);
        Assert.NotNull(storedRepository.LastPrunedAt);
        Assert.Equal(BackupRepositoryStatus.Ready, storedRepository.Status);
    }

    [Fact]
    public async Task CheckAsync_ShouldRejectWhenRepositoryLeaseIsActive()
    {
        var repository = await CreateFileSystemRepositoryAsync(
            "backup-busy",
            Path.Combine(testRoot, "repositories", "busy"));

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var acquired = await uow.BackupRepositoryLeases.TryAcquireAsync(
                repository.Id,
                "Existing",
                Guid.CreateVersion7(),
                DateTimeOffset.UtcNow.AddMinutes(5),
                DateTimeOffset.UtcNow,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
            Assert.True(acquired);
        }

        var service = Services.GetRequiredService<IBackupRepositoryDestinationService>();
        var result = await service.CheckAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess());
        Assert.Contains(result.Errors, error => error.Message == "Backup repository already has an active operation.");
        Assert.Empty(restic.Calls);
    }

    [Fact]
    public async Task ValidateAsync_ShouldBuildS3ResticEnvironment()
    {
        var passwordSecretId = await CreateInternalSecretAsync("RESTIC_S3_PASSWORD", "restic-password");
        var accessKeySecretId = await CreateInternalSecretAsync("RESTIC_S3_ACCESS_KEY", "access-key");
        var secretKeySecretId = await CreateInternalSecretAsync("RESTIC_S3_SECRET_KEY", "secret-key");
        var repository = await CreateRepositoryAsync(
            "backup-s3",
            new S3CompatibleBackupRepositorySpec(
                new Uri("https://minio.example.com"),
                "citadel",
                "daily/core",
                "us-east-1",
                S3BucketLookup.Path,
                accessKeySecretId,
                secretKeySecretId,
                SessionTokenSecretId: null),
            passwordSecretId);
        restic.Enqueue(exitCode: 0, stdout: "[]");

        var service = Services.GetRequiredService<IBackupRepositoryDestinationService>();
        var result = await service.ValidateAsync(
            repository.Id,
            new BackupExecutionContext(BackupExecutionLocation.Core, null),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(), ErrorMessages(result.Errors));

        var call = Assert.Single(restic.Calls);
        Assert.Equal(
            ["-o", "s3.bucket-lookup=path", "snapshots", "--json"],
            call.Command.Arguments);
        Assert.Equal("s3:https://minio.example.com/citadel/daily/core", call.Command.Environment["RESTIC_REPOSITORY"]);
        Assert.Equal("access-key", call.Command.Environment["AWS_ACCESS_KEY_ID"]);
        Assert.Equal("secret-key", call.Command.Environment["AWS_SECRET_ACCESS_KEY"]);
        Assert.Equal("us-east-1", call.Command.Environment["AWS_DEFAULT_REGION"]);
        Assert.Contains("access-key", call.Command.RedactionValues);
        Assert.Contains("secret-key", call.Command.RedactionValues);
    }

    private async Task<BackupRepository> CreateFileSystemRepositoryAsync(string name, string path)
    {
        var passwordSecretId = await CreateInternalSecretAsync($"{name.Replace('-', '_').ToUpperInvariant()}_PASSWORD", "restic-password");
        return await CreateRepositoryAsync(
            name,
            new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, Path.GetFullPath(path)),
            passwordSecretId);
    }

    private async Task<BackupRepository> CreateRepositoryAsync(
        string name,
        BackupRepositorySpec spec,
        Guid passwordSecretId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repository = new BackupRepository(name, null, spec, passwordSecretId, Constants.SystemId);
        await uow.BackupRepositories.AddAsync(repository, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return repository;
    }

    private async Task<Guid> CreateInternalSecretAsync(string name, string value)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name, value },
            cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken);
        using var document = await JsonDocument.ParseAsync(stream, cancellationToken: TestContext.Current.CancellationToken);
        return document.RootElement.GetProperty("id").GetGuid();
    }

    private static string ErrorMessages(IEnumerable<LightResults.IError> errors)
        => string.Join(Environment.NewLine, errors.Select(static error => error.Message));

    private sealed class FakeResticProcessRunner : IResticProcessRunner
    {
        private readonly Queue<ResticResponse> responses = new();

        public List<ResticProcessCall> Calls { get; } = [];

        public void Enqueue(int exitCode, string? stdout = null, string? stderr = null)
            => responses.Enqueue(new ResticResponse(exitCode, stdout, stderr));

        public async IAsyncEnumerable<ResticProcessEvent> RunAsync(
            ResticProcessCommand command,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            var passwordFilePath = command.Environment["RESTIC_PASSWORD_FILE"];
            Calls.Add(new ResticProcessCall(
                command,
                passwordFilePath,
                File.Exists(passwordFilePath),
                File.Exists(passwordFilePath) ? await File.ReadAllTextAsync(passwordFilePath, cancellationToken) : null));

            var response = responses.Count > 0 ? responses.Dequeue() : new ResticResponse(0, "[]", null);
            await Task.Yield();

            if (response.Stdout is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdOut, response.Stdout);

            if (response.Stderr is not null)
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, response.Stderr);

            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: response.ExitCode);
        }
    }

    private sealed record ResticResponse(int ExitCode, string? Stdout, string? Stderr);

    private sealed record ResticProcessCall(
        ResticProcessCommand Command,
        string PasswordFilePath,
        bool PasswordFileExisted,
        string? PasswordFileContent);
}
