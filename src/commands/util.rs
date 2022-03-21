use crate::model::{Job, load_excluded};
use serenity::{
    model::prelude::*,
    prelude::*,
};
use std::collections::{HashMap, HashSet};

/// Returns the voice channel id of the caller.
pub async fn get_callers_vc(ctx: &Context, msg: &Message) -> Option<ChannelId> {
    let guild = msg.guild(ctx).await.unwrap();
    let result = guild.voice_states.get(&msg.author.id);
    if let Some(voice_state) = result {
        return voice_state.channel_id;
    }
    None
}

/// Returns all users in a voice channel.
pub async fn get_members_in_vc(ctx: &Context, msg: &Message, voice_channel_id: ChannelId) -> Vec<Member> {
    let channel = voice_channel_id
        .to_channel(&ctx.http)
        .await
        .unwrap()
        .guild()
        .unwrap();
    let guild_id = msg.guild_id.unwrap();
    let members = channel.members(ctx)
        .await
        .unwrap();
    let mut new_members = Vec::new();
    
    for member in members {
        let member = guild_id.member(&ctx.http, member.user.id).await.unwrap();
        new_members.push(member);
    }
    new_members
}

/// Turns a vector of members into a map of users and their roles.
pub fn get_users_roles(members: Vec<Member>) -> HashMap<UserId, HashSet<RoleId>> {
    members.into_iter().map(|member| {
        (member.user.id, member.roles.into_iter().collect())
    }).collect()
}

/// Remove players with exclude role.
pub fn remove_excluded(
    users_roles: &mut HashMap<UserId, HashSet<RoleId>>
) {
    if let Some(excluded) = load_excluded() {
        let role_id = RoleId(excluded);
        users_roles.retain(|_user_id, roles| {
            roles.get(&role_id).is_none()
        });
    }
}

/// Removes qualifications that don't have jobs.
pub fn remove_irrelevant_qualifications(
    users: &mut HashMap<UserId, HashSet<RoleId>>,
    jobs: &HashMap<u64, Job>,
) {
    for (_id, roles) in users.iter_mut() {
        roles.retain(|role_id| jobs.get(&role_id.0).is_some());
    }
    users.retain(|_user_id, roles| {
        roles.len() > 0
    })
}