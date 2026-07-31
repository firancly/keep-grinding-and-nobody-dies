

enum ButtonColor {
    Red,
    Blue,
    Green,
}


enum EventAction {
    UpsideDown,
    Dyslexia,
    FakeBlueScreen,
    TurkAttack,
    FnafJumpscare,
}

struct Event {
    Name: String,
    Description: String,
    Action : EventAction
}

enum CableColor {
    Red,
    Blue,
    Green,
    White,
    Black,
    Orange,
    Yellow,
}

enum BombAction {
    CutCable(CableColor),
    PressButton(ButtonColor),
}

struct Bomb{
    Action: Vec<BombAction>,
    Phase: u8,
    WrongActions: u8,
    RandomEvent: Event,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
