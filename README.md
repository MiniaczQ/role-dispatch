# Role dispatch
Simple discord bot for assigning roles semi-randomly to people during speedrunning sessions, mostly intended for private use.

When asked, the bot will randomly assign jobs to all people present in the requester's voice chat and post them in a chat.
Jobs can only be assigned to people who are qualified for them (by having specific discord user roles).
Special exclusion role is available for non-participants.
The roles have to fill in a quota which is different depending on the amount of participants.
The roles and amounts can be configured through commands and are stored in `jobs.ron` file.

# Usage
Launching the bot:
 - download release for your platform of choice
 - add your discord bot token as an environmental variable `DISCORD_TOKEN` directly or by providing a `.env` file
 - run the executable

Using the bot:
 - `!help` to see available commands
 - `!help <command>` to see the description and usage of commands

# Discontinued
We no longer use it, there will be no further development or fixes.
