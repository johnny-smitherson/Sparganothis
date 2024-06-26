use leptos::*;


use game::{
    api::websocket::GetAllCustomGames, tet::GameState
};

use crate::websocket::demo_comp::{WebsocketAPI, call_api_sync};
use leptos_struct_table::*;

#[component]
pub fn ListAllCustomGames() -> impl IntoView {
    let all_games = create_rw_signal(vec![]);

    call_api_sync::<GetAllCustomGames>((), Callback::new(move |_r| {
        all_games.set(_r);
    }));

    let table_from_rows = move || {
        let rows = all_games.get();
        let rows = rows
            .iter()
            .map(|r| CustomGameDbRow::new(r.clone()))
            // .filter(|f| f.num_segments > 0)
            .collect::<Vec<_>>();

        view! {
            <table id="get_custom_games">
                <TableContent rows row_renderer=CustomTableRowRenderer/>
            </table>
        }
        .into_view()
    };

    view! { {table_from_rows} }
}


use leptos_struct_table::BootstrapClassesPreset;
#[derive(TableRow, Clone, Debug)]
#[table( 
    classes_provider = "BootstrapClassesPreset", impl_vec_data_provider)]
pub struct CustomGameDbRow {
    pub save_name: String,
    #[table(skip)]
    pub game_state: GameState,
    pub start_time: i64,
}

impl CustomGameDbRow {
    pub fn new(db_row: (String, GameState)) -> Self {
        Self {
            save_name: db_row.0,
            game_state: db_row.1.clone(),
            start_time: db_row.1.start_time,
        }
    }

}




#[allow(unused_variables, non_snake_case)]
pub fn CustomTableRowRenderer(
    // The class attribute for the row element. Generated by the classes provider.
    class: Signal<String>,
    // The row to render.
    row: CustomGameDbRow,
    // The index of the row. Starts at 0 for the first body row.
    index: usize,
    // The selected state of the row. True, when the row is selected.
    selected: Signal<bool>,
    // Event handler callback when this row is selected
    on_select: EventHandler<web_sys::MouseEvent>,
    // Event handler callback for changes
    on_change: EventHandler<ChangeEvent<CustomGameDbRow>>,
) -> impl IntoView {
    let row2 = row.clone();
    let row3 = row.clone();
    view! {
        <tr class=class on:click=move |mouse_event| on_select.run(mouse_event)>
            {row2.render_row(index, on_change)}
            <td>
                <a href=move || {
                    format!("/edit-custom-game/{}", row2.save_name)
                }>Edit</a>
            </td>
            <td>
                <a href=move || {
                    format!("/play-custom-game/{}", row3.save_name)
                }>Play</a>
            </td>
        </tr>
    }
}