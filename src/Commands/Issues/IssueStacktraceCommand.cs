using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

/// <summary>
/// The shortcut that matters when debugging: go from an issue id straight to the stacktrace of its
/// most recent event, without first listing events to find a UUID.
/// </summary>
public sealed class IssueStacktraceCommand : AsyncCommand<SingleIssueSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, SingleIssueSettings settings)
    {
        var client = settings.CreateClient();
        var eventId = await client.ResolveLatestEventIdAsync(settings.Issue);
        System.Console.WriteLine((await client.GetTextAsync($"events/{eventId}/stacktrace/")).TrimEnd());
        return 0;
    }
}
