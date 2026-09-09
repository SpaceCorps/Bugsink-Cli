using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class GetIssueCommand : AsyncCommand<SingleIssueSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, SingleIssueSettings settings)
    {
        var client = settings.CreateClient();
        YamlOutput.Write(await client.GetAsync(settings.Path()));
        return 0;
    }
}
