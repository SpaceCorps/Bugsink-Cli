using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class ResolveIssueCommand : AsyncCommand<ResolveIssueCommand.Settings>
{
    public sealed class Settings : SingleIssueSettings
    {
        [CommandOption("--latest-release")]
        [Description("Resolve in the latest release: reopens if it recurs in a newer one")]
        public bool LatestRelease { get; init; }

        [CommandOption("--next-release")]
        [Description("Resolve in the next release: reopens if it recurs after the next release")]
        public bool NextRelease { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        if (settings.LatestRelease && settings.NextRelease)
            throw new BugsinkApiException("Pass either --latest-release or --next-release, not both.");

        var action = settings switch
        {
            { LatestRelease: true } => "resolve-latest/",
            { NextRelease: true } => "resolve-next/",
            _ => "resolve/"
        };

        var client = settings.CreateClient();
        YamlOutput.Write(await client.PostAsync(settings.Path(action)));
        Status.Ok($"Issue {settings.Issue} resolved.");
        return 0;
    }
}
