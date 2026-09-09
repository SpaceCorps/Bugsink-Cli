using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class UnmuteIssueCommand : AsyncCommand<SingleIssueSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, SingleIssueSettings settings)
    {
        var client = settings.CreateClient();
        YamlOutput.Write(await client.PostAsync(settings.Path("unmute/")));
        Status.Ok($"Issue {settings.Issue} unmuted.");
        return 0;
    }
}
