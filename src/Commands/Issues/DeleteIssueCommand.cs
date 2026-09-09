using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class DeleteIssueCommand : AsyncCommand<SingleIssueSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, SingleIssueSettings settings)
    {
        var client = settings.CreateClient();
        await client.DeleteAsync(settings.Path());
        Status.Ok($"Issue {settings.Issue} deleted.");
        return 0;
    }
}
