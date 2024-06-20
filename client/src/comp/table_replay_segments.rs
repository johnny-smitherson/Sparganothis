use game::tet::{GameReplaySegment, GameReplaySlice, GameState};

use leptos::*;
use leptos_struct_table::*;

#[component]
pub fn TableReplaySegments(
    all_segments: Signal<Vec<GameReplaySegment>>,
    slider: RwSignal<f64>,
    game_state: ReadSignal<GameState>,
) -> impl IntoView {
    let make_table = move || {
        all_segments.with(|all_segments| {
            let total_row_count = all_segments.len();
            let page_size = total_row_count.min(18);
            let pointer = slider.get() as i32;
            let init_row = (pointer - (page_size / 2) as i32).max(0) as usize;
            let game_state = game_state.get();

            let mut rows = all_segments
                .iter()
                .enumerate()
                .skip(init_row)
                .take(page_size)
                .map(|r| GameSegmentTableRow::new(r.0, r.1.clone(), pointer as usize, if r.0 == (pointer as usize) {Some(game_state.clone())} else {None}))
                .collect::<Vec<_>>();
            if rows.len() > 2 {

                for i in 1..rows.len()-1{
                    if let (Some(a), Some(b)) = (rows[i-1].update_slice.clone(), rows[i].update_slice.clone()) {
                        let dt_ns = b.event_timestamp - a.event_timestamp;
                        let dt_ms  = dt_ns as f64/1000.0;
                        rows[i].since_last = format!("{dt_ms:.0}ms")
                    }
                }
            }

            view! { <TableContent rows  row_renderer=CustomTableRowRenderer /> } //
        })
    };
    view! { <table id="table-replay-segments">{make_table}</table> }
}

#[derive(TableRow, Clone, Debug)]
#[table(impl_vec_data_provider)]
pub struct GameSegmentTableRow {
    pub _type: String,
    pub idx: String,
    pub action: String,
    pub since_last: String,
    pub selected: String,
    #[table(skip)]
    pub state: Option<GameState>,
    #[table(skip)]
    pub update_slice: Option<GameReplaySlice>,
}

impl GameSegmentTableRow {
    pub fn new(
        row_idx: usize,
        db_row: GameReplaySegment,
        current_slider: usize,
        state: Option<GameState>
    ) -> Self {
        let selected = if row_idx == current_slider {
            "X".to_string()
        } else {
            "".to_string()
        };
        match db_row {
            GameReplaySegment::Init(_init) => Self {
                _type: "init".to_owned(),
                idx: "".to_owned(),
                action: "".to_owned(),
                since_last: "".to_owned(),
                selected,state, update_slice: None,
            },
            GameReplaySegment::Update(_update) => Self {
                _type: "update".to_owned(),
                idx: _update.idx.to_string(),
                action: format!("{:?}", _update.event.action),
                since_last: "".to_string(),
                selected,state, update_slice: Some(_update),
            },
            GameReplaySegment::GameOver => Self {
                _type: "game_over".to_owned(),
                idx: "".to_owned(),
                action: "".to_owned(),
                since_last: "".to_owned(),
                selected,state, update_slice: None,
            },
        }
    }
}



#[allow(unused_variables, non_snake_case)]
pub fn CustomTableRowRenderer(
    // The class attribute for the row element. Generated by the classes provider.
    class: Signal<String>,
    // The row to render.
    row: GameSegmentTableRow,
    // The index of the row. Starts at 0 for the first body row.
    index: usize,
    // The selected state of the row. True, when the row is selected.
    selected: Signal<bool>,
    // Event handler callback when this row is selected
    on_select: EventHandler<web_sys::MouseEvent>,
    // Event handler callback for changes
    on_change: EventHandler<ChangeEvent<GameSegmentTableRow>>,
) -> impl IntoView {
    let row2 = row.clone();
    let row3 = row.clone();
    let display_state =
        if let Some(state) = row.state {
            view!{
                <tr><td colspan="100%"><code><pre>{format!("{}", state.get_debug_info())}</pre></code></td></tr>
            }.into_view()
        } else {
            view!{}.into_view()
    };
    view! {
        <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
            {row2.render_row(index, on_change)}
        </tr>
        {display_state}
    }
}