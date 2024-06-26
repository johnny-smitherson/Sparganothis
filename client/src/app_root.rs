use futures::StreamExt;
use leptos::*;
use leptos_meta::{provide_meta_context, Meta, Script, Stylesheet, Title};
use leptos_router::*;
// use crate::error_template::ErrorTemplate;
use game::api::websocket::{WebsocketAPIMessageRaw, WhoAmI};
use leptonic::prelude::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::{use_websocket, UseWebsocketReturn};

use crate::comp::game_board_flex::GameBoardFlexDemoPage;
use crate::comp::game_board_mspaint::{MsPaintPage, MsPaintPlayPage};
use crate::hotkey_context::provide_hotkeys_context2;
use crate::page::homepage::Homepage;
use crate::page::page_spectate::SpectateGamePage;
use crate::page::page_match::MatchPage;
use crate::page::page_1p::GameSoloLobbyPage;


#[component]
pub fn AppRoot() -> impl IntoView {
    let _style = stylist::style!(
        nav {
            position: absolute;
            left: 1vmin;
            top: 1vmin;
            height: 97vmin;
            width: 19.2vmin;
            border: 1vmin solid black;
        }
        main {
            position: absolute;
            top: 1vmin;
            left: 19.85vmin;
            height: 98vmin;
        }
        main > div.main_left {
            position: absolute;
            top: 0vmin;
            width: 71.1vmin;
            left: 1.1vmin;
            height: 97vmin;
            border: 1vmin solid green;
        }
        main > div.main_right {
            position: absolute;
            top: 0vmin;
            width: 71.1vmin;
            left: 72.85vmin;
            height: 97vmin;
            border: 1vmin solid blue;
        }
        .menu_root {
            padding: 0px;
        }
        .menu_item {
            margin: 0px;
            height: 6vmin;
            text-align: center;
            line-height: 6vmin;
            font-size: 3vmin;
            font-weight: normal;
            color: black;
            rotate: -11deg;
        }
        a {
            text-decoration: none;
        }
        a[aria-current="page"] > .menu_item  {
            font-weight: bold;
            color: darkred;
            border: 0.5vmin darkred solid;
            margin: 0.5vmin;
            height: 5vmin;
            line-height: 5vmin;
        }
        .profile_view_container {
            color: black;
            margin: 10px;
            border: 8px dashed black;
            padding: 15px;
        }
    )
    .expect("bad css");


    provide_meta_context();

    let main_ref = create_node_ref::<html::Main>();
    provide_hotkeys_context2(main_ref);

    use crate::websocket::demo_comp::*;
    use std::rc::Rc;

    let UseWebsocketReturn {
        ready_state,
        // message,
        message_bytes,
        // send,
        send_bytes,
        open,
        close,
        ..
    } = use_websocket("ws://localhost:3000/api/ws");

    let connected = move || ready_state.get() == ConnectionReadyState::Open;
    let mut ready_state_stream = ready_state.clone().to_stream();
    let ready_signal = create_rw_signal(false);

    let (tx, rx) = async_broadcast::broadcast::<ConnectionReadyState>(1);
    spawn_local(async move {
        loop {
            let r = ready_state_stream.next().await;
            if let Some(r) = r {
                if r.eq(&ConnectionReadyState::Open) {
                    ready_signal.set(true);
                } else {
                    ready_signal.set(false);
                }
                if let Err(e) = tx.broadcast(r).await {
                    log::warn!("error sending to ready stream...: {e:?}");
                } else {
                    log::info!("sent on stream: {:?}", r);
                }
            }
        }
    });

    let open_connection = move |_| {
        log::info!("websocket reopened.");
        open();
    };

    let close_connection = move |_| {
        log::info!("websocket closed intentionally.");
        close();
    };

    // let message =
    // let message = bincode::serialize(&message).unwrap();
    let api = WebsocketAPI {
        map: create_rw_signal(std::collections::HashMap::<_, _>::new()),
        sender: create_rw_signal(Rc::new(Box::new(send_bytes.clone()))),
        ready_state_stream: rx.deactivate(),
        ready_signal,
        subscribe_game_callbacks: create_rw_signal(std::collections::HashMap::<_, _>::new()),
        error_msgs: create_rw_signal(Vec::<_>::new()),
    };
    provide_context(api.clone());

    let send_byte_message = move |_| {
        call_api_sync::<WhoAmI>((), move |r| {
            log::info!("WHO AMM I? {:?}", r);
        });
    };
    let mut recv_bytes_stream = message_bytes.to_stream();
    // let last_message_size = create_rw_signal(0);
    // let last_message_id = create_rw_signal(0);

    log::info!("console init");
    let api_spawn = api.clone();
    spawn_local(async move {
        log::info!("spawn local init");
        use futures::stream::StreamExt;
        loop {
            while let Some(Some(c)) = recv_bytes_stream.next().await {
                // last_message_size.set_untracked(c.len() as i32);
                // last_message_id.set_untracked(last_message_id.try_get_untracked().unwrap_or(0) % 999);
                // log::debug!("websocket got {} bytes", c.len());
                match bincode::deserialize::<WebsocketAPIMessageRaw>(&c) {
                    Ok(msg) => {
                        // log::info!("recv message type={:?} len={}", msg._type, c.len(),);
                        if msg.is_req {
                            if let Err(e) = accept_subscribe_notification(&api_spawn.clone(), msg).await {
                                log::warn!("error accepting subscribe notifgication: {:?}", e);
                            }
                        } else {
                            accept_reply_message(&api_spawn.clone(), msg).await;
                        }
                        // let ctx = expect_context::<RwSignal<WebsocketAPI>>();
                        // log::info!("successfully got global context size={}!", ctx.get_untracked().map.len());
                    }
                    Err(e) => {
                        log::warn!("websocket deserialize error {:?}", e);
                    }
                }
            }
            log::info!("websocket reciever died.");
            // thread::sleep(std::time::Duration::from_millis(3));
        }
    });

    let status = move || {
        let st = ready_state.get();
        log::info!("websocket status: {}", st);
        match st {
            ConnectionReadyState::Open => {
                view! { <h1 style="color:darkgreen">{st.to_string()}</h1> }.into_view()
            },
            _  => {
                view! { <h1 style="color:red">{st.to_string()}</h1> }.into_view()
            },
        }
    };

    use crate::page::page_1p::Game1PPage;
    use crate::page::page_2p_lobby::Game2LobbyPage;
    use crate::page::page_user_profile::{MyAccountPage, UserProfilePage};
    use crate::page::page_vs_cpu::GameCPUPage;

    view! {
        <Meta name="charset" content="UTF-8"/>
        <Meta name="description" content="FALLING BLOCKS"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Meta name="theme-color" content="#e66956"/>

        <Script src="/public/jquery-3.7.1.min.js"/>
        <Script src="/public/bootstrap.min.js"/>
        <Stylesheet id="bootstrap" href="/public/bootstrap.min.css"/>
        <Stylesheet id="bootstrap-extra" href="/public/bootstrap_extra.css"/>

        <leptos_meta::Link
            rel="icon"
            type_="image/x-icon"
            href="/public/favicon.ico"
        ></leptos_meta::Link>
        // <Stylesheet href="https://fonts.googleapis.com/css?family=Roboto&display=swap"/>

        <Title text="xanthoides"/>

        <Root default_theme=LeptonicTheme::default()>

            <div class=_style.get_class_name().to_string()>

                <Router fallback=|| {
                    let mut outside_errors = Errors::default();
                    outside_errors
                        .insert_with_default_key(
                            crate::error_template::AppError::NotFound,
                        );
                    view! {
                        <crate::error_template::ErrorTemplate outside_errors></crate::error_template::ErrorTemplate>
                    }
                }>
                    <nav>
                        <MainMenu/>
                        <div>
                            <p>"status: " {status}</p>

                            <button
                                on:click=send_byte_message
                                disabled=move || !connected()
                            >
                                "Send bytes"
                            </button>
                            <button on:click=open_connection disabled=connected>
                                "Open"
                            </button>
                            <button
                                on:click=close_connection
                                disabled=move || !connected()
                            >
                                "Close"
                            </button>

                            // <p>{sig}</p>
                            <p>
                                {move || {
                                    format!(
                                        "{:?} bytes",
                                        message_bytes.get().unwrap_or(vec![]).len(),
                                    )
                                }}

                            </p>
                        </div>
                    </nav>
                    <main _ref=main_ref>
                        // all our routes will appear inside <main>
                        <Routes>
                            <Route path="" view=Homepage/>
                            <Route path="/solo" view=GameSoloLobbyPage/>
                            <Route path="/play-game-solo/:game_id" view=Game1PPage/>
                            <Route path="/vs_cpu" view=GameCPUPage/>
                            <Route path="/vs_net" view=Game2LobbyPage/>
                            <Route
                                path="/replay"
                                view=crate::page::page_replay_browser::GameReplayBrowserPage
                            />
                            <Route path="/account" view=MyAccountPage/>
                            <Route path="/spectate-game/:game_id" view=SpectateGamePage/>
                            <Route path="/user/:user_id" view=UserProfilePage/>
                            <Route
                                path="/view-game/:game_id"
                                view=crate::page::page_replay_single::GameReplaySinglePage
                            />
                            <Route path="/match/:match_id" view=MatchPage/>
                            <Route path="/mspaint" view=MsPaintPage/>
                            <Route path="/edit-custom-game/:save_id" view=MsPaintPage/>
                            <Route
                                path="/play-custom-game/:save_id"
                                view=MsPaintPlayPage
                            />
                            <Route path="/gamebordflex" view=GameBoardFlexDemoPage/>

                        </Routes>
                    </main>
                </Router>
            </div>
        </Root>
    }
}

#[component]
pub fn MainMenu() -> impl IntoView {
    let menu_entries = || {
        vec![
            ("/", "home"),
            ("/solo", "solo"),
            ("/vs_cpu", "man vs car"),
            ("/vs_net", "1v1 online"),
            ("/replay", "replay"),
            ("/account", "account"),
            ("/mspaint", "mspaint"),
            ("/gamebordflex", "gamebordflex"),
        ]
    };
    // let git_version = create_resource(
    //     || (),
    //     |_| async move { crate::server::api::server_info::git_version().await },
    // );

    view! {
        <ul class="menu_root">
            <For
                each=menu_entries
                key=|k| k.0
                children=|k| {
                    view! {
                        <A href=k.0>
                            <h3 class="menu_item">{k.1}</h3>
                        </A>
                    }
                }
            />

        </ul>
    }
}
