use crate::model::{load_jobs, save_jobs, Job};
use indexmap::{IndexMap, IndexSet};
use rand::{distributions::Uniform, prelude::Distribution};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    time::Duration,
};

#[command]
pub async fn roll(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let anchor = msg
        .reply(ctx, "React with the dice to randomize roles!")
        .await
        .unwrap();
    anchor
        .react(ctx, ReactionType::Unicode("🎲".to_owned()))
        .await
        .unwrap();
    anchor
        .react(ctx, ReactionType::Unicode("❌".to_owned()))
        .await
        .unwrap();

    let mut response: Option<Message> = None;

    loop {
        let result = anchor
            .await_reaction(&ctx)
            .timeout(Duration::from_secs(60 * 60))
            .await;

        if let Some(msg) = response {
            msg.delete(ctx).await.unwrap();
            response = None;
        }

        if let Some(reaction) = result {
            let reaction = reaction.as_inner_ref();
            let emoji = &reaction.emoji;
            match emoji.as_data().as_str() {
                "🎲" => {
                    response = try_assigning(ctx, &msg, &anchor.channel_id).await;
                }
                "❌" => break,
                _ => (),
            };
            reaction.delete(ctx).await.unwrap();
        } else {
            break;
        };
    }

    anchor.delete(ctx).await.unwrap();
    if let Some(msg) = response {
        msg.delete(ctx).await.unwrap();
    }

    Ok(())
}

/// Decide role for every participating player
async fn try_assigning(ctx: &Context, msg: &Message, channel_id: &ChannelId) -> Option<Message> {
    let guild = &msg.guild(ctx).await.unwrap();
    msg.delete(ctx).await.ok();
    if let Some(voice_channel_id) = get_vc(guild, &msg.author.id).await {
        let jobs = get_jobs(guild);
        let players = get_players(ctx, guild, voice_channel_id, &jobs).await;
        let assigned = decide_pairings(&jobs, &players);
        return Some(display_pairings(ctx, guild, channel_id, &jobs, assigned).await);
    }
    None
}

/// Returns the voice channel id of the user.
async fn get_vc(guild: &Guild, author_id: &UserId) -> Option<ChannelId> {
    let result = guild.voice_states.get(author_id);
    if let Some(voice_state) = result {
        return voice_state.channel_id;
    }
    None
}

/// Returns all players in a voice channel.
fn get_players_in_vc(guild: &Guild, voice_channel_id: ChannelId) -> Vec<UserId> {
    let mut user_ids: Vec<UserId> = Vec::new();
    for (user_id, voice_state) in guild.voice_states.iter() {
        if let Some(other_id) = voice_state.channel_id {
            if other_id == voice_channel_id {
                user_ids.push(user_id.clone());
            }
        }
    }
    user_ids
}

/// Returns all players roles.
async fn get_player_roles(
    ctx: &Context,
    guild: &Guild,
    player_ids: Vec<UserId>,
) -> HashMap<UserId, HashSet<RoleId>> {
    //guild.id.member(&ctx.http, UserId::from(0)).await.unwrap().roles;
    let mut players_roles = HashMap::new();
    let pg = guild.id.to_partial_guild(&ctx.http).await.unwrap();
    for player_id in player_ids {
        let member = pg.member(&ctx.http, &player_id).await.unwrap();
        players_roles.insert(
            player_id,
            member
                .roles(ctx)
                .await
                .unwrap()
                .into_iter()
                .map(|role| role.id)
                .collect(),
        );
    }
    players_roles
}

/// Removes qualifications that don't have jobs.
fn remove_irrelevant_qualifications(
    players: &mut HashMap<UserId, HashSet<RoleId>>,
    jobs: &HashMap<RoleId, Job>,
) {
    for (_id, roles) in players.iter_mut() {
        roles.retain(|e| jobs.get(e).is_some());
    }
}

/// All server roles.
fn get_role_names(guild: &Guild) -> HashMap<RoleId, String> {
    let mut role_names = HashMap::new();
    for (role_id, role) in guild.roles.iter() {
        role_names.insert(*role_id, role.name.clone());
    }
    role_names
}

/// Jobs mapped by their corresponding role id.
fn get_jobs(guild: &Guild) -> HashMap<RoleId, Job> {
    let role_names = get_role_names(guild);
    load_jobs()
        .into_iter()
        .filter_map(|(key, job)| {
            match role_names
                .iter()
                .find(|(_role_id, name)| name.to_lowercase().eq(&key))
            {
                Some((role_id, _name)) => Some((*role_id, job)),
                None => None,
            }
        })
        .collect()
}

/// Players with only useful qualifications.
async fn get_players(
    ctx: &Context,
    guild: &Guild,
    voice_channel_id: ChannelId,
    jobs: &HashMap<RoleId, Job>,
) -> HashMap<UserId, HashSet<RoleId>> {
    let player_ids = get_players_in_vc(&guild, voice_channel_id);
    let mut players = get_player_roles(ctx, guild, player_ids).await;
    remove_irrelevant_qualifications(&mut players, jobs);
    players
}

#[derive(Debug)]
struct JobExt {
    at_least: u32,
    assigned: u32,
    players: IndexSet<UserId>,
}

/// Decide pairings.
/// Decide how many of each role based on the proportions and limits
/// Finish by applying last step of Hungarian Algorithm.
/// Returns error if there was a problem during assignment.
fn decide_pairings(
    jobs: &HashMap<RoleId, Job>,
    players: &HashMap<UserId, HashSet<RoleId>>,
) -> Result<IndexMap<RoleId, Vec<UserId>>, String> {
    /// Variables required for assigning jobs.
    #[derive(Debug)]
    struct AssigningJob {
        needed: u32,
        leftovers: f64,
        players: IndexSet<UserId>,
    }

    /// Variables required for assigning players.
    #[derive(Debug)]
    struct AssigningPlayer {}

    /// Assigns a player to a job.
    /// Update all the variables.
    /// Remove used players and jobs.
    fn assign(
        assigned: &mut IndexMap<RoleId, Vec<UserId>>,
        left_jobs: &mut IndexMap<RoleId, AssigningJob>,
        left_players: &mut IndexMap<UserId, AssigningPlayer>,
        role_id: RoleId,
        user_id: UserId,
    ) {
        // Update `assigned`
        if let Some(vec) = match assigned.get_mut(&role_id) {
            Some(vec) => {
                vec.push(user_id);
                None
            }
            None => {
                let mut vec = Vec::new();
                vec.push(user_id);
                Some(vec)
            }
        } {
            assigned.insert(role_id, vec);
        };
        // Update `left_jobs`
        let job = left_jobs.get_mut(&role_id).unwrap();
        job.needed -= 1;
        if job.needed <= 0 {
            left_jobs.remove(&role_id);
        }
        left_jobs.iter_mut().for_each(|(_role_id, job)| {
            job.players.remove(&user_id);
        });
        // Update `left_players`
        left_players.remove(&user_id);
    }

    // Initialize assignment variables
    let mut assigned: IndexMap<RoleId, Vec<UserId>> = IndexMap::new();
    let mut left_jobs: IndexMap<RoleId, AssigningJob> = jobs
        .iter()
        .map(|(role_id, job)| {
            (
                *role_id,
                AssigningJob {
                    needed: job.minimum,
                    leftovers: 0.,
                    players: players
                        .iter()
                        .filter_map(|(user_id, roles)| {
                            roles.get(role_id).is_some().then_some(*user_id)
                        })
                        .collect(),
                },
            )
        })
        .collect();
    let mut for_assignment: IndexMap<RoleId, AssigningJob> = IndexMap::new();
    let rng = &mut rand::thread_rng();

    //{
    //    println!("Step 1");
    //    println!("Left jobs:");
    //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //    println!("For assignment:");
    //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //}

    // Calculate leftovers
    let mut players_left = players.len() as u32;
    let req_for_minimums = left_jobs
        .iter()
        .map(|(_role_id, job)| job.needed)
        .sum::<u32>();

    //{
    //    println!("Step 2");
    //    println!("Left jobs:");
    //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //    println!("For assignment:");
    //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //}
    
    // Remove maxed jobs
    left_jobs = left_jobs
        .into_iter()
        .filter_map(|(role_id, job)| {
            if jobs.get(&role_id).unwrap().maximum <= job.needed {
                for_assignment.insert(role_id, job);
                None
            } else {
                Some((role_id, job))
            }
        })
        .collect();
    
    //{
    //    println!("Step 3");
    //    println!("Left jobs:");
    //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //    println!("For assignment:");
    //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //}
    
    // Check if there is enough players to meet the minimum
    if req_for_minimums > players_left {
        return Err("Not enough players to satisfy the minimal requirements.".to_owned());
    }
    players_left -= req_for_minimums;
    let mut pool = players_left as f64;
    while pool > 0.00001 {
        // Check if no jobs left, maximums are too low
        if left_jobs.len() == 0 {
            return Err("Too many players to satisfy the maximal requirements.".to_owned());
        }
        //{
        //    println!("Step 4a");
        //    println!("Left jobs:");
        //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //    println!("For assignment:");
        //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //}
        // Assign leftovers
        let normalizer: f64 = left_jobs
            .iter()
            .map(|(role_id, _job)| jobs.get(role_id).unwrap().proportion)
            .sum();
        left_jobs.iter_mut().for_each(|(role_id, job)| {
            job.leftovers += pool * jobs.get(role_id).unwrap().proportion / normalizer;
        });
        pool = 0.;
        //{
        //    println!("Step 4b");
        //    println!("Left jobs:");
        //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //    println!("For assignment:");
        //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //}
        // Reduce to maximums
        left_jobs = left_jobs
            .into_iter()
            .filter_map(|(role_id, mut job)| {
                let maximum = jobs.get(&role_id).unwrap().maximum;
                let available = (maximum - job.needed) as f64;
                if available < job.leftovers {
                    pool += job.leftovers - available;
                    job.leftovers = 0.;
                    players_left -= maximum - job.needed;
                    job.needed = maximum;
                    for_assignment.insert(role_id, job);
                    None
                } else {
                    let change = job.leftovers.trunc() as u32;
                    players_left -= change;
                    job.needed += change;
                    job.leftovers = job.leftovers.fract();
                    Some((role_id, job))
                }
            })
            .collect();
        
        //{
        //    println!("Step 4c");
        //    println!("Left jobs:");
        //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //    println!("For assignment:");
        //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
        //}
    }
    //{
    //    println!("Step 5");
    //    println!("Left jobs:");
    //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //    println!("For assignment:");
    //    for_assignment.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //}
    // Distribute based on the remaining fractions
    // Check if too many players
    if left_jobs.len() < players_left as usize {
        return Err("Too many players to satisfy the maximal requirements.".to_owned());
    }
    for _ in 0..players_left {
        let (role_id, _job) = left_jobs
            .iter_mut()
            .max_by(|(_role_id_a, job_a), (_role_id_b, job_b)| {
                job_a.leftovers.total_cmp(&job_b.leftovers)
            })
            .unwrap();
        let role_id = *role_id;
        let mut job = left_jobs.remove(&role_id).unwrap();
        job.leftovers = 0.;
        job.needed += 1;
        for_assignment.insert(role_id, job);
    }
    left_jobs.into_iter().for_each(|(role_id, mut job)| {
        job.leftovers = 0.;
        for_assignment.insert(role_id, job);
    });
    let mut left_jobs = for_assignment;
    //{
    //    println!("Step 6");
    //    println!("Left jobs:");
    //    left_jobs.iter().for_each(|(role_id, job)| {println!("{}: {:?}", jobs.get(role_id).unwrap().name, job)});
    //}
    // Initialize more assignment variables
    let mut left_players: IndexMap<UserId, AssigningPlayer> = players
        .iter()
        .map(|(user_id, _roles)| (*user_id, AssigningPlayer {}))
        .collect();
    // Assign players to roles
    for _ in 0..left_players.len() {
        let (role_id, job) = left_jobs
            .iter()
            .filter(|(_role_id, job)| job.needed > 0)
            .min_by_key(|(_role_id, job)| job.players.len())
            .unwrap();
        let role_id = *role_id;
        let available = job.players.len();
        // Check if any players are available
        if available == 0 {
            return Err(format!(
                "Not enough players qualified for role: {}",
                jobs.get(&role_id).unwrap().name
            ));
        }
        let user_id = *job
            .players
            .get_index(Uniform::from(0..available).sample(rng))
            .unwrap();
        assign(
            &mut assigned,
            &mut left_jobs,
            &mut left_players,
            role_id,
            user_id,
        );
    }
    Ok(assigned)
}

/// Displays the distribution of roles or the error.
async fn display_pairings(
    ctx: &Context,
    guild: &Guild,
    channel_id: &ChannelId,
    jobs: &HashMap<RoleId, Job>,
    assigned: Result<IndexMap<RoleId, Vec<UserId>>, String>,
) -> Message {
    let channel = guild.channels.get(channel_id).unwrap();
    let content = match assigned {
        Ok(mut assigned) => {
            let mut content = String::new();
            assigned.sort_keys();
            assigned.into_iter().for_each(|(role_id, players)| {
                content
                    .write_fmt(format_args!("**{}**:\n", jobs.get(&role_id).unwrap().name))
                    .ok();
                players.into_iter().for_each(|user_id| {
                    content.write_fmt(format_args!("> <@{}>\n", user_id)).ok();
                });
                content.write_str("\n").ok();
            });
            content
        }
        Err(s) => s,
    };

    channel
        .send_message(ctx, |m| m.content(content))
        .await
        .unwrap()
}

#[command]
pub async fn modify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let response = match parse_args_to_job(args) {
        Ok(job) => {
            let mut jobs = load_jobs();
            let job_name = job.name.clone();
            let response = match jobs.insert(job.name.to_ascii_lowercase(), job) {
                Some(previous_job) => {
                    let role = guild.role_by_name(&previous_job.name).unwrap();
                    role.edit(ctx, |e| e.name(job_name)).await.ok();
                    "Role modified succesfully!".to_owned()
                }
                None => {
                    guild.create_role(ctx, |e| e.name(job_name)).await.ok();
                    "Role added succesfully!".to_owned()
                }
            };
            save_jobs(jobs);
            response
        }
        Err(e) => e,
    };

    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel
        .send_message(ctx, |m| m.content(response))
        .await
        .ok();

    Ok(())
}

fn parse_args_to_job(mut args: Args) -> Result<Job, String> {
    args.trimmed().quoted();
    let name = match args.single::<String>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid name".to_owned()),
    };
    let minimum = match args.single::<u32>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid minimum (non-negative integer)".to_owned()),
    };
    let maximum = match args.single::<u32>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid maximum (non-negative integer)".to_owned()),
    };
    let proportion = match args.single::<f64>() {
        Ok(proportion) => proportion,
        Err(_) => return Err("Invalid proportion (non-negative)".to_owned()),
    };
    return Ok(Job {
        name,
        minimum,
        maximum,
        proportion,
    });
}

#[command]
pub async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    args.trimmed().quoted();
    let response = match args.single::<String>() {
        Ok(name) => {
            let mut jobs = load_jobs();
            let response = match jobs.remove(&name.to_lowercase()) {
                Some(job) => {
                    let role = guild.role_by_name(&job.name).unwrap();
                    guild.delete_role(ctx, role.id).await.ok();
                    "Role removed succesfully!".to_owned()
                }
                None => "No role with that name exists.".to_owned(),
            };
            save_jobs(jobs);
            response
        }
        Err(_) => "Invalid name".to_owned(),
    };

    let guild = msg.guild(ctx).await.unwrap();
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel
        .send_message(ctx, |m| m.content(response))
        .await
        .ok();

    Ok(())
}

#[command]
pub async fn show(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let jobs = load_jobs();
    let mut response = String::new();
    response.write_str("**Existing roles:**\n\n").ok();
    for (_key, job) in jobs {
        response
            .write_fmt(format_args!(
                "***{}***\n> **Minimum**: {}\n> **Maximum**: {}\n> **Proportion**: {}\n\n",
                job.name, job.minimum, job.maximum, job.proportion
            ))
            .ok();
    }

    let guild = msg.guild(ctx).await.unwrap();
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel
        .send_message(ctx, |m| m.content(response))
        .await
        .ok();

    Ok(())
}

#[command]
pub async fn grant(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let jobs = load_jobs();
    let mut member = msg.member(ctx).await.unwrap();
    let mut invalid = Vec::<String>::new();
    args.trimmed().quoted();
    for _ in 0..args.len() {
        let name = args.single::<String>().unwrap();
        match jobs.get(&name.to_ascii_lowercase()) {
            Some(_) => match guild.role_by_name(&name) {
                Some(role) => member.add_role(ctx, role.id).await.unwrap(),
                None => invalid.push(name),
            },
            None => {
                invalid.push(name);
            }
        }
    }

    let response = match invalid.len() {
        0 => "Roles were granted succesfully!".to_owned(),
        _ => {
            let mut response = String::new();
            for s in invalid.iter() {
                response
                    .write_fmt(format_args!("{} is not a valid role.\n", s))
                    .ok();
            }
            if args.len() != invalid.len() {
                response
                    .write_str("The rest of the roles were granted succesfully!")
                    .ok();
            }
            response
        }
    };

    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel
        .send_message(ctx, |m| m.content(response))
        .await
        .ok();

    Ok(())
}

#[command]
pub async fn revoke(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let jobs = load_jobs();
    let mut member = msg.member(ctx).await.unwrap();
    let mut invalid = Vec::<String>::new();
    args.trimmed().quoted();
    for _ in 0..args.len() {
        let name = args.single::<String>().unwrap();
        match jobs.get(&name.to_ascii_lowercase()) {
            Some(_) => match guild.role_by_name(&name) {
                Some(role) => member.remove_role(ctx, role.id).await.unwrap(),
                None => invalid.push(name),
            },
            None => {
                invalid.push(name);
            }
        }
    }

    let response = match invalid.len() {
        0 => "Roles were revoked succesfully!".to_owned(),
        _ => {
            let mut response = String::new();
            for s in invalid.iter() {
                response
                    .write_fmt(format_args!("{} is not a valid role.\n", s))
                    .ok();
            }
            if args.len() != invalid.len() {
                response
                    .write_str("The rest of the roles were revoked succesfully!")
                    .ok();
            }
            response
        }
    };

    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel
        .send_message(ctx, |m| m.content(response))
        .await
        .ok();

    Ok(())
}

#[command]
pub async fn simulate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();

    args.trimmed().quoted();
    let content = match args.single::<u32>() {
        Ok(count) => {
            let jobs = get_jobs(&guild);
            let players = simulate_players(&jobs, count);
            match decide_pairings(&jobs, &players) {
                Ok(mut assigned) => {
                    let mut content = String::new();
                    content.write_str("Simulation succedeed:\n").ok();
                    assigned.sort_keys();
                    assigned.into_iter().for_each(|(role_id, players)| {
                        content
                            .write_fmt(format_args!(
                                "**{}**: {}\n",
                                jobs.get(&role_id).unwrap().name,
                                players.len()
                            ))
                            .ok();
                    });
                    content
                }
                Err(s) => {
                    let mut content = String::new();
                    content
                        .write_fmt(format_args!("Simulation failed:\n{}", s))
                        .ok();
                    content
                }
            }
        }
        Err(_) => "Invalid amount (non-negative integer)".to_owned(),
    };

    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| m.content(content)).await.ok();

    Ok(())
}

fn simulate_players(jobs: &HashMap<RoleId, Job>, count: u32) -> HashMap<UserId, HashSet<RoleId>> {
    let mut players = HashMap::new();
    for i in 0..count {
        players.insert(
            UserId::from(i as u64),
            jobs.iter().map(|(role_id, _job)| *role_id).collect(),
        );
    }
    players
}
