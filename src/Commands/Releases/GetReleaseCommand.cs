using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Releases;

public sealed class GetReleaseCommand : AsyncCommand<GetReleaseCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<RELEASE>")]
        [Description("Release UUID, as listed by 'releases list'")]
        public required string Release { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        YamlOutput.Write(await client.GetAsync($"releases/{Uri.EscapeDataString(settings.Release)}/"));
        return 0;
    }
}
