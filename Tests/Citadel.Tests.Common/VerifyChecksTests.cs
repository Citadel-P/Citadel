using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using Argon;
using DiffEngine;

namespace Tests.Common;

public class VerifyChecksTests
{
    [Fact]
    public Task Initialize() => VerifyChecks.Run();
}

public static class VerifyModuleInitializer
{
    [ModuleInitializer]
    public static void Initialize()
    {
        // Configure the directory for snapshots
        UseProjectRelativeDirectory("Snapshots");

        // Configure the diff tool to use
        DiffRunner.Disabled = false;
        DiffTools.UseOrder(DiffTool.VisualStudio, DiffTool.VisualStudioCode, DiffTool.Rider);

        // Configure the snapshot serializer settings
        VerifierSettings.AddExtraSettings(settings => settings.DefaultValueHandling = DefaultValueHandling.Include);

        // Scrub traceId from the snapshots, as it's not needed for verification
        VerifierSettings.ScrubMembers("traceId");
    }
}
