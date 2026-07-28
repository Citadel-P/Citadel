using Domain.Contracts.Interfaces;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using System.Threading.Channels;

namespace Infrastructure.Repositories;

internal sealed partial class ResticProcessRunner : IResticProcessRunner
{
    public async IAsyncEnumerable<ResticProcessEvent> RunAsync(
        ResticProcessCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        Directory.CreateDirectory(command.WorkingDirectory);

        using var timeoutCts = new CancellationTokenSource(command.Timeout);
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);
        var ct = linkedCts.Token;
        var redactionValues = PrepareRedactionValues(command.RedactionValues);

        var startInfo = new ProcessStartInfo
        {
            FileName = command.FileName,
            WorkingDirectory = command.WorkingDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        foreach (var argument in command.Arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        foreach (var kv in command.Environment)
        {
            startInfo.Environment[kv.Key] = kv.Value;
        }

        using var process = new Process
        {
            StartInfo = startInfo,
            EnableRaisingEvents = true
        };

        var channel = Channel.CreateBounded<ResticProcessEvent>(
            new BoundedChannelOptions(256)
            {
                SingleReader = true,
                SingleWriter = false,
                FullMode = BoundedChannelFullMode.Wait
            });

        string? startError = null;
        try
        {
            process.Start();
        }
        catch (Exception ex)
        {
            startError = Sanitize(ex.Message, command.MaxLineBytes, redactionValues);
        }

        if (startError is not null)
        {
            yield return new ResticProcessEvent(ResticProcessStream.StdErr, startError);
            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: -1);
            yield break;
        }

        var stdoutTask = PumpAsync(
            process.StandardOutput,
            ResticProcessStream.StdOut,
            channel.Writer,
            command.MaxLineBytes,
            redactionValues,
            ct);
        var stderrTask = PumpAsync(
            process.StandardError,
            ResticProcessStream.StdErr,
            channel.Writer,
            command.MaxLineBytes,
            redactionValues,
            ct);
        var completionTask = CompleteAsync(process, stdoutTask, stderrTask, channel.Writer, ct);

        try
        {
            await foreach (var item in channel.Reader.ReadAllAsync(cancellationToken))
            {
                yield return item;
            }

            await completionTask;
        }
        finally
        {
            await linkedCts.CancelAsync();
            TryKill(process);
            channel.Writer.TryComplete();
            await ObserveCompletionAsync(completionTask);
        }

        if (timeoutCts.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
        {
            yield return new ResticProcessEvent(ResticProcessStream.StdErr, "Restic operation timed out.");
            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: -2);
        }
    }

    private static async Task CompleteAsync(
        Process process,
        Task stdoutTask,
        Task stderrTask,
        ChannelWriter<ResticProcessEvent> writer,
        CancellationToken cancellationToken)
    {
        try
        {
            await Task.WhenAll(stdoutTask, stderrTask);
            await process.WaitForExitAsync(cancellationToken);
            await writer.WriteAsync(
                new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: process.ExitCode),
                cancellationToken);
        }
        catch (OperationCanceledException)
        {
            TryKill(process);
        }
        finally
        {
            writer.TryComplete();
        }
    }

    private static async Task PumpAsync(
        StreamReader reader,
        ResticProcessStream stream,
        ChannelWriter<ResticProcessEvent> writer,
        int maxLineBytes,
        IReadOnlyList<string> redactionValues,
        CancellationToken cancellationToken)
    {
        while (true)
        {
            string? line;
            try
            {
                line = await reader.ReadLineAsync(cancellationToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }

            if (line is null)
                break;

            await writer.WriteAsync(
                new ResticProcessEvent(stream, Sanitize(line, maxLineBytes, redactionValues)),
                cancellationToken);
        }
    }

    private static string Sanitize(
        string value,
        int maxLineBytes,
        IReadOnlyList<string> redactionValues)
    {
        var sanitized = value.IndexOf('\x1B') >= 0
            ? AnsiRegex().Replace(value, string.Empty)
            : value;
        foreach (var secret in redactionValues)
        {
            if (sanitized.Contains(secret, StringComparison.Ordinal))
                sanitized = sanitized.Replace(secret, "********", StringComparison.Ordinal);
        }

        return sanitized.Length <= maxLineBytes
            ? sanitized
            : sanitized[..maxLineBytes] + "...";
    }

    private static string[] PrepareRedactionValues(IEnumerable<string> redactionValues)
        => [.. redactionValues
            .Where(static value => !string.IsNullOrEmpty(value))
            .Distinct(StringComparer.Ordinal)
            .OrderByDescending(static value => value.Length)];

    private static async Task ObserveCompletionAsync(Task completionTask)
    {
        try
        {
            await completionTask;
        }
        catch (OperationCanceledException)
        {
        }
        catch (ChannelClosedException)
        {
        }
        catch
        {
        }
    }

    private static void TryKill(Process process)
    {
        try
        {
            if (!process.HasExited)
                process.Kill(entireProcessTree: true);
        }
        catch
        {
        }
    }

    [GeneratedRegex(@"\x1B\[[0-?]*[ -/]*[@-~]", RegexOptions.Compiled)]
    private static partial Regex AnsiRegex();
}
