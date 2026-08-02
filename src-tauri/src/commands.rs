use std::sync::Mutex;

use tauri::State;

use crate::engine::GameState;
use crate::view::DefuserView;

pub struct EngineState(pub Mutex<GameState>);

#[tauri::command]
pub fn get_game_state(state: State<'_, EngineState>) -> Option<DefuserView> {
    state.0.lock().unwrap().last_view.clone()
}

#[tauri::command]
pub fn restart_game(state: State<'_, EngineState>) {
    // Resetting here is enough - the background engine loop notices the
    // change (fresh state has no cached last_view) on its next tick and
    // emits + resends the display value on its own.
    *state.0.lock().unwrap() = GameState::new();
}
