using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

public sealed class CommentIssueCommand : AsyncCommand<CommentIssueCommand.Settings>
{
    public sealed class Settings : SingleIssueSettings
    {
        [CommandOption("--text <TEXT>")]
        [Description("Comment body")]
        public string? Text { get; init; }

        [CommandOption("--stdin")]
        [Description("Read the comment body from stdin instead of --text")]
        public bool Stdin { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        if (settings.Stdin == (settings.Text is not null))
            throw new BugsinkApiException("Pass exactly one of --text <TEXT> or --stdin.");

        var text = settings.Stdin
            ? await System.Console.In.ReadToEndAsync()
            : settings.Text!;

        if (string.IsNullOrWhiteSpace(text))
            throw new BugsinkApiException("Comment text is empty.");

        var client = settings.CreateClient();
        var body = new Dictionary<string, object?>
        {
            ["issue"] = settings.Issue,
            ["comment"] = text.TrimEnd()
        };

        YamlOutput.Write(await client.PostAsync("issue-comments/", body));
        Status.Ok($"Comment added to {settings.Issue}.");
        return 0;
    }
}
