use game::{
    api::{
        game_replay::GameId,
        websocket::{GameSegmentCountReply, GetAllGames},
    },
    random::GameSeed,
    timestamp::get_human_readable_nano,
};

use crate::websocket::demo_comp::{_call_websocket_api, WebsocketAPI};
use game::api::websocket::GetAllGamesArg;
use leptos::*;
use leptos_struct_table::*;


#[component]
pub fn AllGamesTable(list_type: GetAllGamesArg) -> impl IntoView {
    let api2: WebsocketAPI = expect_context();
    let all_games = create_resource(
        || (),
        move |_| {
            let api2 = api2.clone();
            async move {
                // log::info!("calling websocket api");
                let r = _call_websocket_api::<GetAllGames>(api2, list_type)
                    .expect("cannot obtain future")
                    .await;
                // log::info!("got back response: {:?}", r);
                r
            }
        },
    );

    let table_from_rows = move || {
        if let Some(Ok(rows)) = all_games.get() {
            let rows = rows
                .iter()
                .map(|r| FullGameReplayTableRow::new(r.clone()))
                // .filter(|f| f.num_segments > 0)
                .collect::<Vec<_>>();

            view! {
                <table id=format!("{list_type:?}")>
                    <TableContent rows row_renderer=CustomTableRowRenderer/>
                </table>
            }
            .into_view()
        } else {
            view! { <p>loading...</p> }.into_view()
        }
    };

    view! { {table_from_rows} }
}

#[allow(unused_variables, non_snake_case)]
pub fn CustomTableRowRenderer(
    // The class attribute for the row element. Generated by the classes provider.
    class: Signal<String>,
    // The row to render.
    row: FullGameReplayTableRow,
    // The index of the row. Starts at 0 for the first body row.
    index: usize,
    // The selected state of the row. True, when the row is selected.
    selected: Signal<bool>,
    // Event handler callback when this row is selected
    on_select: EventHandler<web_sys::MouseEvent>,
    // Event handler callback for changes
    on_change: EventHandler<ChangeEvent<FullGameReplayTableRow>>,
) -> impl IntoView {
    let row2 = row.clone();
    let row3 = row.clone();
    view! {
        <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
            {row2.render_row(index, on_change)}
            <td>
                <a href=move || {
                    if row3.is_in_progress {
                        format!("/spectate-game/{}", row.to_url())
                    } else {
                        format!("/view-game/{}", row.to_url())
                    }
                }>
                    {move || {
                        if row3.is_in_progress {
                            "Spectate".to_string()
                        } else {
                            "Replay".to_string()
                        }
                    }}

                </a>
            </td>
        </tr>
    }
}

use leptos_struct_table::BootstrapClassesPreset;

#[derive(TableRow, Clone, Debug)]
#[table( 
    classes_provider = "BootstrapClassesPreset", impl_vec_data_provider)]
pub struct FullGameReplayTableRow {
    #[table(renderer = "WeedRenderer")]
    pub user_id: uuid::Uuid,
    #[table(renderer = "SeedRenderer")]
    pub init_seed: GameSeed,
    #[table(renderer = "TimeRenderer")]
    pub start_time: i64,
    pub num_segments: usize,
    pub is_in_progress: bool,
}

impl FullGameReplayTableRow {
    pub fn new(db_row: (GameId, GameSegmentCountReply)) -> Self {
        Self {
            user_id: db_row.0.user_id,
            init_seed: db_row.0.init_seed,
            start_time: db_row.0.start_time,
            num_segments: db_row.1.segment_count as usize,
            is_in_progress: db_row.1.is_in_progress,
        }
    }

    pub fn to_url(&self) -> String {
        GameId {
            user_id: self.user_id,
            init_seed: self.init_seed,
            start_time: self.start_time,
        }
        .to_url()
    }
}

#[allow(unused_variables)]
#[component]
fn TimeRenderer<F>(
    class: String,
    #[prop(into)] value: MaybeSignal<i64>,
    on_change: F,
    index: usize,
) -> impl IntoView
where
    F: Fn(i64) + 'static,
{
    view! {
        <td class=class>
            <p>{move || { get_human_readable_nano(value.get()) }}</p>
        </td>
    }
}

#[allow(unused_variables)]
#[component]
fn WeedRenderer<F>(
    class: String,
    #[prop(into)] value: MaybeSignal<uuid::Uuid>,
    on_change: F,
    index: usize,
) -> impl IntoView
where
    F: Fn(uuid::Uuid) + 'static,
{
    view! {
        <td class=class>
            <a href=format!("/user/{:?}", value.get())>
                <p style="border: 1px solid black">
                    {move || { format!("{:?}", value.get())[0..8].to_string() }}
                </p>
            </a>
        </td>
    }
}

#[allow(unused_variables)]
#[component]
fn SeedRenderer<F>(
    class: String,
    #[prop(into)] value: MaybeSignal<GameSeed>,
    on_change: F,
    index: usize,
) -> impl IntoView
where
    F: Fn(GameSeed) + 'static,
{
    view! {
        <td class=class>
            <p>{move || format!("{:?}, ..", value.get()[0])}</p>
        </td>
    }
}
