use crate::backend::server_info::GIT_VERSION;
use crate::database::tables::*;

use anyhow::Context;
use game::api::game_match::GameMatch;
use game::api::game_match::GameMatchType;
use game::api::game_replay::GameId;
use game::api::game_replay::GameSegmentId;
use game::api::user::GuestInfo;
use game::api::user::UserProfile;
use game::api::websocket::GameSegmentCountReply;
use game::api::websocket::GetMatchListArg;
use game::tet::GameReplaySegment;
use game::tet::GameState;
use game::timestamp::get_timestamp_now_nano;
use rand::Rng;

pub fn get_profile(
    user_id: uuid::Uuid,
    _current_user_id: GuestInfo,
) -> anyhow::Result<UserProfile> {
    use crate::database::tables::get_user_profile;
    get_user_profile(&user_id)
}

pub fn git_version(_: (), _current_user_id: GuestInfo) -> anyhow::Result<String> {
    Ok(GIT_VERSION.clone())
}

pub fn create_new_game_id(
    _: (),
    _current_user_id: GuestInfo,
) -> anyhow::Result<GameId> {
    for existing_game in GAME_IS_IN_PROGRESS_DB
        .range(GameId::get_range_for_user(&_current_user_id.user_id))
    {
        let (old_game_id, is_in_progress) = existing_game?;
        if is_in_progress {
            GAME_IS_IN_PROGRESS_DB.insert(&old_game_id, &false)?;
        }
    }

    let mut rand = rand::thread_rng();
    let g = GameId {
        user_id: _current_user_id.user_id,
        init_seed: rand.gen(),
        start_time: get_timestamp_now_nano(),
    };

    GAME_IS_IN_PROGRESS_DB.insert(&g, &true)?;
    GAME_SEGMENT_COUNT_DB.insert(&g, &0)?;
    Ok(g)
}

pub fn append_game_segment(
    (id, segment_json): (GameId, String),
    _current_user_id: GuestInfo,
) -> anyhow::Result<()> {
    let new_segment: GameReplaySegment =
        serde_json::from_str(&segment_json).expect("json never fail");

    let who = _current_user_id.user_id;
    if !who.eq(&id.user_id) {
        anyhow::bail!("no impersonate plz");
    }

    let existing_segment_count = GAME_SEGMENT_COUNT_DB
        .get(&id)?
        .context("game segment count not found!")?;
    let last_segment: Option<GameReplaySegment> = if existing_segment_count > 0 {
        let old_segment_id = GameSegmentId {
            game_id: id,
            segment_id: existing_segment_count - 1,
        };
        let maybe_segment =
            GAME_SEGMENT_DB.get(&old_segment_id)?.context("not found")?;
        Some(maybe_segment)
    } else {
        None
    };
    let last_state: Option<GameState> = if existing_segment_count > 0 {
        let maybe_gamestate = GAME_FULL_DB.get(&id)?.context("not found")?;
        Some(maybe_gamestate)
    } else {
        None
    };

    let new_segment_id = GameSegmentId {
        game_id: id,
        segment_id: existing_segment_count,
    };

    match &new_segment {
        GameReplaySegment::Init(_) => {
            if existing_segment_count != 0 {
                anyhow::bail!("only 1st segment should be init");
            }
        }
        GameReplaySegment::Update(update_seg) => {
            let last_segment = last_segment.context("last segment not found")?;
            match last_segment {
                GameReplaySegment::Init(_) => {
                    if update_seg.idx != 0 {
                        anyhow::bail!("1st update segmnet needs idx=0");
                    }
                }
                GameReplaySegment::Update(old_update) => {
                    if old_update.idx + 1 != update_seg.idx {
                        anyhow::bail!(
                            "segment idx do not match up - missing/duplicate"
                        );
                    }
                }
                GameReplaySegment::GameOver => {
                    anyhow::bail!("already have old segmnet for game over");
                }
            }
        }
        GameReplaySegment::GameOver => {
            log::info!("append segment game over");
        }
    };
    let game_in_progress = match &new_segment {
        GameReplaySegment::Init(_) => true,
        GameReplaySegment::Update(_) => true,
        GameReplaySegment::GameOver => false,
    };
    GAME_IS_IN_PROGRESS_DB.insert(&id, &game_in_progress)?;
    GAME_SEGMENT_DB.insert(&new_segment_id, &new_segment)?;
    GAME_SEGMENT_COUNT_DB.insert(&id, &(existing_segment_count + 1))?;

    let new_game_state = match new_segment {
        GameReplaySegment::Init(replay) => {
            GameState::new(&replay.init_seed, replay.start_time)
        }
        GameReplaySegment::Update(slice) => {
            let mut last_state = last_state.context("no last state found")?;
            last_state.accept_replay_slice(&slice)?;
            last_state
        }
        GameReplaySegment::GameOver => {
            let last_state = last_state.context("no last state found")?;
            if !last_state.game_over {
                anyhow::bail!("got game over but reconstructed state is not game over")
            }
            last_state
        }
    };
    GAME_FULL_DB.insert(&id, &new_game_state)?;

    Ok(())
}

pub fn get_last_full_game_state(
    game_id: GameId,
    _current_user_id: GuestInfo,
) -> anyhow::Result<Option<GameState>> {
    Ok(GAME_FULL_DB.get(&game_id)?)
}

pub fn get_all_segments_for_game(
    game_id: GameId,
    _current_user_id: GuestInfo,
) -> anyhow::Result<Vec<GameReplaySegment>> {
    let mut r = vec![];
    for item in GAME_SEGMENT_DB
        .range(GameSegmentId::get_range_for_game(&game_id))
        .into_iter()
    {
        let (_segment_id, replay_segment) = item?;
        r.push(replay_segment);
    }
    r.sort_by_key(|s| match s {
        GameReplaySegment::Init(_) => -1,
        GameReplaySegment::Update(_s) => _s.idx as i32,
        GameReplaySegment::GameOver => i32::MAX,
    });
    Ok(r)
}

pub fn get_segment_count(
    game_id: GameId,
    _current_user_id: GuestInfo,
) -> anyhow::Result<GameSegmentCountReply> {
    let is_in_progress = GAME_IS_IN_PROGRESS_DB
        .get(&game_id)?
        .context("not fgound")?;
    let seg_count = GAME_SEGMENT_COUNT_DB.get(&game_id)?.context("not found")?;
    Ok(GameSegmentCountReply {
        is_in_progress,
        segment_count: seg_count,
    })
}
use game::api::websocket::GetAllGamesArg;
const PAGE_SIZE: usize = 9;

pub fn get_all_games(
    arg: GetAllGamesArg,
    _current_user_id: GuestInfo,
) -> anyhow::Result<Vec<(GameId, GameSegmentCountReply)>> {
    let load_all_games = || -> anyhow::Result<_> {
        let mut v = vec![];
        for game_id in GAME_IS_IN_PROGRESS_DB.iter().keys() {
            let game_id = game_id?;
            let r = get_segment_count(game_id, _current_user_id.clone())?;
            v.push((game_id, r));
        }
        Ok(v)
    };
    let load_games_for_user = |user: &uuid::Uuid| -> anyhow::Result<_> {
        let mut v = vec![];
        for game_id in GAME_IS_IN_PROGRESS_DB
            .range(GameId::get_range_for_user(user))
            .keys()
        {
            let game_id = game_id?;
            let r = get_segment_count(game_id, _current_user_id.clone())?;
            v.push((game_id, r));
        }
        Ok(v)
    };
    let sort_best = |mut v: Vec<(_, GameSegmentCountReply)>| -> anyhow::Result<_> {
        v.sort_by_key(|x| -(x.1.segment_count as i32));
        Ok(v)
    };
    let sort_recent = |mut v: Vec<(GameId, _)>| -> anyhow::Result<_> {
        v.sort_by_key(|x| -((x.0.start_time / 100000) as i32));
        Ok(v)
    };

    let mut v = match arg {
        GetAllGamesArg::BestGames => sort_best(load_all_games()?)?,
        GetAllGamesArg::RecentGames => sort_recent(load_all_games()?)?,
        GetAllGamesArg::MyBestGames => {
            sort_best(load_games_for_user(&_current_user_id.user_id)?)?
        }
        GetAllGamesArg::MyRecentGames => {
            sort_recent(load_games_for_user(&_current_user_id.user_id)?)?
        }
        GetAllGamesArg::BestGamesForPlayer(player_id) => {
            sort_best(load_games_for_user(&player_id)?)?
        }
        GetAllGamesArg::RecentGamesForPlayer(player_id) => {
            sort_recent(load_games_for_user(&player_id)?)?
        }
    };
    v.truncate(PAGE_SIZE);
    Ok(v)
}

#[allow(unused_variables)]
pub fn get_all_gustom(
    arg: (),
    _current_user_id: GuestInfo,
) -> anyhow::Result<Vec<(String, GameState)>> {
    let mut v = vec![];
    for x in CUSTOM_GAME_BOARD_DB.iter() {
        let x = x?;
        v.push(x);
    }
    Ok(v)
}

pub fn get_gustom_game(
    arg: String,
    _current_user_id: GuestInfo,
) -> anyhow::Result<GameState> {
    Ok(CUSTOM_GAME_BOARD_DB.get(&arg)?.context("not found")?)
}

pub fn update_custom_game(
    arg: (String, GameState),
    _current_user_id: GuestInfo,
) -> anyhow::Result<()> {
    CUSTOM_GAME_BOARD_DB.insert(&arg.0, &arg.1)?;
    Ok(())
}

pub fn random_word2(_: (), _current_user_id: GuestInfo) -> anyhow::Result<String> {
    Ok(random_word())
}

pub struct MatchMakingItem {
    channel: tokio::sync::mpsc::Sender<(uuid::Uuid, GameMatch)>,
    player_id: uuid::Uuid,
}

pub static MATCH_MAKING_QUEUE: Lazy<MatchMakingQueue> =
    Lazy::new(|| MatchMakingQueue {
        v: Arc::new(Mutex::new(HashMap::new())),
    });

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
pub struct MatchMakingQueue {
    v: Arc<Mutex<HashMap<uuid::Uuid, MatchMakingItem>>>,
}

pub async fn start_match(
    _: GameMatchType,
    _current_user_id: GuestInfo,
) -> anyhow::Result<(uuid::Uuid, GameMatch)> {
    // Ok(uuid::Uuid::nil(), GameMatch)

    let mut _waiting_for_match: Option<_> = None;
    let mut _got_new_match: Option<_> = None;
    {
        let mut q = MATCH_MAKING_QUEUE.v.lock().await;
        if q.is_empty() {
                // creezi chan, te bagi in el
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                let player_id =  _current_user_id.user_id;
                let new_item = MatchMakingItem {
                    channel: tx,
                    player_id ,
                };

                _waiting_for_match = Some(rx);
                q.insert(player_id, new_item);
        }else {
            if q.contains_key(&_current_user_id.user_id) {
                anyhow::bail!("another game is already in matchmaking!");
            } else {
                let k = *q.keys().next().unwrap();
                let other_player = q.remove(&k).unwrap();
                let new_match = GameMatch {
                    seed: (&mut rand::thread_rng()).gen(),
                    time: get_timestamp_now_nano(),
                    users: vec![other_player.player_id, _current_user_id.user_id],
                    title: format!(
                        "1v1 {} vs. {}",
                        other_player.player_id, _current_user_id.user_id
                    ),
                };
                let new_match_id = uuid::Uuid::new_v4();
                GAME_MATCH_DB.insert(&new_match_id, &new_match)?;
                other_player
                    .channel
                    .send((new_match_id, new_match.clone()))
                    .await?;
                _got_new_match = Some((new_match_id, new_match));
            }
        }
    }
    if let Some(mut waiting_rx) = _waiting_for_match {
        if let Some(match_info) = waiting_rx.recv().await {
            create_db_match_entry(&match_info.1)?;
            Ok(match_info)
        } else {
            anyhow::bail!("cannot read from channel");
        }
    } else {
        let r = _got_new_match.context("never happens")?;

        create_db_match_entry(&r.1)?;

        Ok(r)
    }
}

fn create_db_match_entry(match_info: &GameMatch) -> anyhow::Result<()> {
    let gameinfo_0 = GameId {
        user_id: match_info.users[0],
        init_seed: match_info.seed,
        start_time: match_info.time,
    };
    let gameinfo_1 = GameId {
        user_id: match_info.users[1],
        init_seed: match_info.seed,
        start_time: match_info.time,
    };

    GAME_IS_IN_PROGRESS_DB.insert(&gameinfo_0, &true)?;
    GAME_SEGMENT_COUNT_DB.insert(&gameinfo_0, &0)?;

    GAME_IS_IN_PROGRESS_DB.insert(&gameinfo_1, &true)?;
    GAME_SEGMENT_COUNT_DB.insert(&gameinfo_1, &0)?;

    Ok(())
}

pub fn get_match_list(
    _: GetMatchListArg,
    _current_user_id: GuestInfo,
) -> anyhow::Result<Vec<(uuid::Uuid, GameMatch)>> {
    let mut v = vec![];
    for x in GAME_MATCH_DB.iter() {
        let (uuid, _match) = x?;
        v.push((uuid, _match));
    }
    Ok(v)
}

pub fn get_match_info(
    match_id: uuid::Uuid,
    _current_user_id: GuestInfo,
) -> anyhow::Result<GameMatch> {
    GAME_MATCH_DB.get(&match_id)?.context(".not found")
}
