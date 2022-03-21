use crate::model::{load_excluded, save_excluded};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[description("Toggle between being exluded and included in role distribution.")]
async fn exclude(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let role_id = match load_excluded() {
        Some(excluded) => RoleId(excluded),
        None => {
            let role_id = guild_id
                .create_role(&ctx.http, |e| e.name("Excluded"))
                .await
                .unwrap()
                .id;
            save_excluded(role_id.0);
            role_id
        }
    };

    let has_role = msg
        .author
        .has_role(&ctx.http, guild_id, role_id)
        .await
        .unwrap();
    let mut member = msg.member(&ctx.http).await.unwrap();

    match has_role {
        true => {
            member.remove_role(&ctx.http, role_id).await.ok();
            msg.channel_id
                .say(
                    &ctx.http,
                    "You will be included in role distribution.".to_owned(),
                )
                .await
                .ok();
        }
        false => {
            member.add_role(&ctx.http, role_id).await.ok();
            msg.channel_id
                .say(
                    &ctx.http,
                    "You won't be included in role distribution.".to_owned(),
                )
                .await
                .ok();
        }
    }

    Ok(())
}
