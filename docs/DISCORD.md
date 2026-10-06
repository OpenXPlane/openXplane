# Discord

openXplane has two Discord features: Rich Presence in the app, and a community link on the website.

## Rich Presence

While the viewer is open, your Discord status can show what you are looking at (for example "Viewing an
aircraft - Cessna_172SP", with the elapsed time). It talks to the Discord desktop app on the same computer over
its local IPC socket (a named pipe on Windows) and sends nothing anywhere else.

openXplane uses its own Discord application (id `1556989566160994374`, public; it only gives the name Discord
shows after "Playing"), so nothing needs to be set. To use a different application:

1. Open the [Discord developer portal](https://discord.com/developers/applications) and create an application.
   The application name is what Discord shows after "Playing".
2. Copy its **Application ID** (a number).
3. Set it before starting openXplane:

   ```sh
   # macOS / Linux
   export OPENXPLANE_DISCORD_APP_ID=123456789012345678
   cargo run --offline -- view "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
   ```

   ```powershell
   # Windows PowerShell
   $env:OPENXPLANE_DISCORD_APP_ID = "123456789012345678"
   cargo run --offline -- view "Xplane12\Cessna 172 SP\Cessna_172SP.acf"
   ```

Set the variable to an empty value to turn presence off. If Discord is not running, openXplane prints
`discord: no running Discord client found` and continues normally. Discord must be the desktop app, running as
the same user.

## Community link

The website shows a Discord button when an invite link is set in `site/app.js` (the `DISCORD` constant); while it
is empty the button is hidden. The invite link has to be created by whoever owns the server.

## Limits

The IPC client is written against Discord's documented RPC frame format and is tested against a local mock
socket. It has not been run against a live Discord client in this repository. Only an activity with a title, a
subtitle and a start time is sent; there are no images, buttons, party or join features.
