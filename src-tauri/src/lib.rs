use std::{collections::HashMap, sync::Mutex};
use tokio_util::sync::CancellationToken;
use tauri::{AppHandle, Manager, State};
use rand::{
    Rng, RngExt, distr::{Distribution, StandardUniform},
};

#[derive(Debug, Clone, Default)]
struct AppState {
    Bomb: HashMap<String, Bomb>,
    TH1CANCELTOKEN: CancellationToken,
}

#[derive(Clone, Debug, Default)]
enum BombType {
    #[default]
    Alpha,
    Beta,
    Omega,
}

#[derive(Clone, Debug, Default)]
enum BombPower {
    #[default]
    Battery,
    Nuclear,
    Electric,
}

#[derive(Clone, Debug, Default)]
enum EventAction {
    #[default]
    UpsideDown,
    Dyslexia,
    FakeBlueScreen,
    TurkAttack,
    FnafJumpscare,
}

#[derive(Clone, Debug)]
enum CableColor {
    Red,
    Blue,
    Green,
    White,
    Black,
    Orange,
    Yellow,
}

#[derive(Clone, Debug)]
enum ButtonColor {
    Red,
    Blue,
    Green,
}

#[derive(Clone, Debug)]
enum BombAction {
    CutCable(CableColor),
    PressButton(ButtonColor),
}

enum RandType {
    BombType,
    BombPower,
    BombTemperature,
    BombAction,
    Event,
}

#[derive(Clone, Debug, Default)]
struct Event {
    Name: String,
    Description: String,
    Action: EventAction,
}

#[derive(Clone, Default, Debug)]
struct Bomb {
    Action: Vec<BombAction>,
    RandomValues: RandomValues,
    WrongActions: u8,
}

#[derive(Clone, Debug, Default)]
struct RandomValues {
    BombType: BombType,
    BombPower: BombPower,
    BombTemperature: u8,
}


impl Distribution<BombType> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombType {
        match rng.random_range(0..3) {
            0 => BombType::Alpha,
            1 => BombType::Beta,
            _ => BombType::Omega,
        }
    }
}

impl Distribution<BombPower> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombPower {
        match rng.random_range(0..3) {
            0 => BombPower::Battery,
            1 => BombPower::Nuclear,
            _ => BombPower::Electric,
        }
    }
}

impl Distribution<CableColor> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CableColor {
        match rng.random_range(0..7) {
            0 => CableColor::Red,
            1 => CableColor::Blue,
            2 => CableColor::Green,
            3 => CableColor::White,
            4 => CableColor::Black,
            5 => CableColor::Orange,
            _ => CableColor::Yellow,
        }
    }
}

impl Distribution<ButtonColor> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ButtonColor {
        match rng.random_range(0..3) {
            0 => ButtonColor::Red,
            1 => ButtonColor::Blue,
            _ => ButtonColor::Green,
        }
    }
}

impl Distribution<BombAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombAction {
        if rng.random_bool(0.5) {
            BombAction::CutCable(rng.random())
        } else {
            BombAction::PressButton(rng.random())
        }
    }
}

impl Distribution<EventAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EventAction {
        match rng.random_range(0..5) {
            0 => EventAction::UpsideDown,
            1 => EventAction::Dyslexia,
            2 => EventAction::FakeBlueScreen,
            3 => EventAction::TurkAttack,
            _ => EventAction::FnafJumpscare,
        }
    }
}

impl Distribution<Event> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Event {
        let action: EventAction = rng.random();
        let (name, desc) = match action {
            EventAction::UpsideDown => ("Upside Down", "Screen is upside down!"),
            EventAction::Dyslexia => ("Dyslexia", "Texts are mixed up!"),
            EventAction::FakeBlueScreen => ("Fake Blue Screen", "System looks crashed!"),
            EventAction::TurkAttack => ("Turk Attack", "There is a cyber attack from turks on the system!"),
            EventAction::FnafJumpscare => ("Fnaf Jumpscare", "A scary image appeared!"),
        };

        Event {
            Name: name.to_string(),
            Description: desc.to_string(),
            Action: action,
        }
    }
}


#[derive(Debug, Clone)]
enum RandResult {
    BombType(BombType),
    BombPower(BombPower),
    BombTemperature(u8),
    BombAction(BombAction),
    Event(Event),
}

async fn random(random_type: RandType) -> Result<RandResult, String> {
    let mut rng = rand::rng();

    let result = match random_type {
        RandType::BombType => RandResult::BombType(rng.random()),
        RandType::BombPower => RandResult::BombPower(rng.random()),
        RandType::BombTemperature => RandResult::BombTemperature(rng.random_range(0..=100)),
        RandType::Event => RandResult::Event(rng.random()),
        RandType::BombAction => RandResult::BombAction(rng.random()),
    };

    Ok(result)
}

impl Bomb {
    async fn generate() -> Self {
        let mut rng = rand::rng();

        let action_count = rng.random_range(3..=6);
        let actions: Vec<BombAction> = (0..action_count).map(|_| rng.random()).collect();

        Bomb {
            Action: actions,
            RandomValues: RandomValues {
                BombType: rng.random(),
                BombPower: rng.random(),
                BombTemperature: rng.random_range(0..=100),
            },
            WrongActions: 0,
        }
    }


    async fn start(&mut self, handle: AppHandle) {
        let cancellation_token = CancellationToken::new();

        let worker_token = cancellation_token.clone();
        let worker_handle = handle.clone();
        let bomb_clone = self.clone();

        tokio::spawn(async move {
            handle_main(bomb_clone, worker_token, worker_handle).await;
        });

        let state = handle.state::<Mutex<AppState>>();
        let mut app_state = state.lock().unwrap();
        app_state.Bomb.insert(format!("APP"), self.clone());
        app_state.TH1CANCELTOKEN = cancellation_token;

    }

}

async fn handle_main(Bomb: Bomb, cancellationtoken : CancellationToken, handle : AppHandle) {
    // Handle the main bomb logic here
}

async fn handle_event(event: Event, handle: AppHandle) {
    // Handle the event logic here
}

#[tauri::command]
async fn start_game() -> Result<(), String> {
    



    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app|{
            let app_handle = app.handle();
            app.manage(Mutex::new(AppState::default()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
