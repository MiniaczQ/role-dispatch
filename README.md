# Role dispatch
Simple discord bot for assigning roles semi-randomly to people during speedrunning sessions, mostly intended for private use.

When asked, the bot will assign roles to all people present in the requester's voice chat.
Roles can only be assigned to people who are qualified for them.
Special exclusion role is available for non-participants.
The roles have to fill in a quota which is different dependant on the amount of participants.
The roles and amounts can be configured through commands and are stored in `jobs.ron` file.

# Usage
Launching the bot:
 - download release for your platform of choice
 - add your discord bot token as an environmental variable `DISCORD_TOKEN` directly or by providing a `.env` file
 - run the executable

Using the bot:
 - `!help` to see all the available commands
 - `!help <command>` to see the description of a command
 - `!<command> <args>` to run the command

# Discontinued
We no longer use it, there will be no further development or fixes.
